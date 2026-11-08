//! One account's connection: on the first start (or a rebuilt mailbox) the existing mail is
//! queued for review (all unread plus the last scan_existing_days days); then new mail is
//! handled as it comes (IDLE, or polling), old mail in batches between, and Achaja's folders are
//! purged delete_after_days after the move. Errors reconnect with a growing pause; a refused
//! login is said aloud once and retried every 30 minutes.

use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};

use super::handle::handle;
use super::Ctx;
use crate::json::{now, read_json, write_json};
use crate::mail::accounts::Account;
use crate::mail::imap::{connect, imap_date, Imap};

const IDLE_RENEW: Duration = Duration::from_secs(600);
const BATCH: usize = 10;
const BACKLOG_BATCH: usize = 20;

pub async fn watch_account(ctx: Arc<Ctx>, a: Account) {
    let mut backoff = 30u64;
    let mut spoken_auth = false;
    loop {
        match run(&ctx, &a, &mut spoken_auth).await {
            Ok(()) => backoff = 30,
            Err(e) => {
                let text = e.to_string();
                ctx.log(&format!("[{}] blad: {text}", a.name));
                ctx.set_status(&a.name, json!({"state": "error", "error": crate::text::truncate_chars(&text, 300)}));
                let auth = text.to_lowercase().contains("authenticat") || text.to_lowercase().contains("login");
                if auth && !spoken_auth {
                    spoken_auth = true;
                    ctx.queue_text(if ctx.lang() == "pl" {
                        format!("Nie mogę zalogować się do poczty {}. Sprawdź hasło w konfiguracji.", a.name)
                    } else {
                        format!("I can't log in to the mailbox {}. Check the password in the config.", a.name)
                    });
                }
                tokio::time::sleep(Duration::from_secs(if auth { 1800 } else { backoff })).await;
                backoff = (backoff * 2).min(600);
            }
        }
    }
}

fn state_file(ctx: &Ctx, a: &Account) -> std::path::PathBuf {
    ctx.paths.state().join("mail").join(format!("account-{}.json", a.safe_name()))
}

async fn plan_backlog(ctx: &Ctx, a: &Account, c: &mut Imap, st: &mut Value, days: Option<u64>, all: bool) -> crate::error::Result<()> {
    let last = st["last_uid"].as_u64().unwrap_or(0) as u32;
    let mut uids: Vec<u32> = if all { c.search("ALL").await? } else { c.search("UNSEEN").await? };
    if let (false, Some(d)) = (all, days.filter(|d| *d > 0)) {
        let since = chrono::Local::now().date_naive() - chrono::Duration::days(d as i64);
        uids.extend(c.search(&format!("SINCE {}", imap_date(since))).await?);
    }
    uids.retain(|u| *u <= last);
    uids.sort_unstable_by(|x, y| y.cmp(x)); // newest first
    uids.dedup();
    st["backlog_total"] = json!(uids.len());
    st["backlog_started"] = json!(now());
    if !uids.is_empty() {
        ctx.set_status(&a.name, json!({"state": "przeglada stara poczte", "backlog": uids.len()}));
    }
    st["backlog"] = json!(uids);
    Ok(())
}

fn backlog(st: &Value) -> Vec<u32> {
    st["backlog"].as_array().map(|a| a.iter().filter_map(|v| v.as_u64().map(|u| u as u32)).collect()).unwrap_or_default()
}

async fn run(ctx: &Arc<Ctx>, a: &Account, spoken_auth: &mut bool) -> crate::error::Result<()> {
    let mut c = connect(a).await?;
    *spoken_auth = false;
    let inbox = c.server_folder(&a.folder()).await;
    let mailbox = c.select(&inbox).await?;
    let file = state_file(ctx, a);
    let mut st: Value = read_json(&file, json!({}));
    if st["uidvalidity"].as_u64() != mailbox.uid_validity.map(u64::from) {
        let existing = c.search("ALL").await?;
        st = json!({"uidvalidity": mailbox.uid_validity, "last_uid": existing.iter().max().copied().unwrap_or(0)});
        if a.b("scan_existing", true) {
            plan_backlog(ctx, a, &mut c, &mut st, Some(a.n("scan_existing_days", 30.0) as u64), false).await?;
        }
        write_json(&file, &st)?;
        ctx.log(&format!("[{}] start od UID {}, stara poczta do przegladu: {}", a.name, st["last_uid"], backlog(&st).len()));
    }
    let idle = c.has("IDLE");
    ctx.log(&format!("[{}] polaczono ({})", a.name, if idle { "IDLE".into() } else { format!("odpytywanie co {} s", a.n("poll_seconds", 120.0)) }));
    let mut last_purge = 0.0;
    loop {
        // a rescan asked with `achaja mail rescan`
        let rescan_file = ctx.paths.state().join("mail").join("rescan.json");
        let mut req: Value = read_json(&rescan_file, json!({}));
        if let Some(r) = req.as_object_mut().and_then(|m| m.remove(&a.name)) {
            write_json(&rescan_file, &req)?;
            let days = r["days"].as_u64().or(Some(a.n("scan_existing_days", 30.0) as u64));
            plan_backlog(ctx, a, &mut c, &mut st, days, r["all"].as_bool().unwrap_or(false)).await?;
            write_json(&file, &st)?;
            ctx.log(&format!("[{}] przeglad na zadanie: {} wiadomosci", a.name, backlog(&st).len()));
        }
        // new mail first, repeated until nothing new arrived meanwhile
        loop {
            c.session.noop().await.map_err(crate::mail::imap::err)?;
            let last = st["last_uid"].as_u64().unwrap_or(0) as u32;
            let new: Vec<u32> = c.search(&format!("UID {}:*", last + 1)).await?.into_iter().filter(|u| *u > last).collect();
            if new.is_empty() {
                break;
            }
            let chunk: Vec<u32> = new.into_iter().take(BATCH).collect();
            handle(ctx, a, &mut c, &inbox, &chunk, false).await?;
            st["last_uid"] = json!(chunk[chunk.len() - 1]);
            write_json(&file, &st)?;
        }
        let old = backlog(&st);
        if !old.is_empty() {
            let chunk: Vec<u32> = old.iter().take(BACKLOG_BATCH).copied().collect();
            handle(ctx, a, &mut c, &inbox, &chunk, true).await?;
            let left: Vec<u32> = old.into_iter().skip(chunk.len()).collect();
            st["backlog"] = json!(left);
            write_json(&file, &st)?;
            ctx.set_status(&a.name, json!({"state": "przeglada stara poczte", "backlog": left.len(), "last_check": now()}));
            if left.is_empty() {
                super::handle::backlog_report(ctx, a, st["backlog_started"].as_f64().unwrap_or(0.0));
            }
            continue;
        }
        if now() - last_purge > 6.0 * 3600.0 {
            last_purge = now();
            super::handle::purge(ctx, a, &mut c, &inbox).await?;
        }
        ctx.set_status(&a.name, json!({"state": "czuwa", "last_check": now(), "error": null, "backlog": 0}));
        if idle {
            let name = a.name.clone();
            let paths = ctx.paths.clone();
            c = c.idle(IDLE_RENEW, move || read_json::<Value>(&paths.state().join("mail").join("rescan.json"), json!({}))[&name].is_object()).await?;
        } else {
            tokio::time::sleep(Duration::from_secs_f64(a.n("poll_seconds", 120.0))).await;
        }
    }
}
