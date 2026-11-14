//! Sorting one batch of messages: the user's earlier decision for mail that came back, the rules,
//! then the model; moving them on the server; logging the metadata; and the purge of Achaja's
//! folders after delete_after_days (counted from the move, not from the date of the mail).

use std::collections::BTreeMap;

use super::classify::classify;
use super::rules::verdict;
use super::Ctx;
use crate::error::Result;
use crate::json::now;
use crate::mail::accounts::Account;
use crate::mail::imap::{flags_of, imap_date, Imap};
use crate::mail::log::{self, Entry, USER_FIX};
use crate::mail::parse::{parse, Mail};
use crate::mail::report::{label, report, ReportOptions};

/// The category the user chose for a message that is back in the inbox, or None.
fn user_decision(prev: Option<&Entry>, inbox: &str) -> Option<String> {
    let prev = prev?;
    if !prev.folder.is_empty() && prev.folder != inbox && prev.folder != "usunieta" {
        return Some("normal".into()); // it lay in another folder and the user dragged it back
    }
    (prev.user_fixed || prev.reason == USER_FIX).then(|| prev.category.clone())
}

pub async fn handle(ctx: &Ctx, a: &Account, c: &mut Imap, inbox: &str, uids: &[u32], backlog: bool) -> Result<()> {
    let fetched = c.fetch(uids, "(BODY.PEEK[] FLAGS)").await?;
    let seen = log::last_by_message_id(&ctx.paths, &a.name);
    let mut parsed: Vec<(u32, Mail)> = Vec::new();
    let mut decided: BTreeMap<u32, (String, String, bool)> = BTreeMap::new();
    let mut for_model: Vec<(String, Mail)> = Vec::new();
    for f in &fetched {
        let (Some(uid), Some(body)) = (f.uid, f.body()) else { continue };
        let mut m = parse(body);
        m.unread = !flags_of(f).iter().any(|fl| fl.contains("Seen"));
        if let Some(cat) = user_decision(seen.get(&m.message_id), inbox) {
            // the user already decided; the same rule would send it back again
            decided.insert(uid, (cat, USER_FIX.into(), true));
        } else {
            let v = tokio::task::block_in_place(|| verdict(&ctx.paths, &m, a));
            if let Some(v) = &v {
                decided.insert(uid, (v.category.to_string(), v.reason.clone(), false));
            }
            if v.as_ref().map_or(true, |v| v.summarize) {
                for_model.push((uid.to_string(), m.clone()));
            }
        }
        parsed.push((uid, m));
    }
    let verdicts = tokio::task::block_in_place(|| classify(&for_model, a, ctx.lang(), "mail", &|t| ctx.log(t)));
    let mut entries = Vec::new();
    let mut groups: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    for (uid, m) in &parsed {
        let model = verdicts.get(&uid.to_string());
        let (category, reason, fixed) = match (decided.get(uid), model) {
            (Some(d), _) => d.clone(),
            (None, Some(v)) => (v.category.clone(), v.reason.clone(), false),
            (None, None) => ("normal".into(), "brak oceny modelu - zostawione w skrzynce".into(), false),
        };
        let mut summary = model.map(|v| v.summary.clone()).unwrap_or_default();
        if fixed && summary.is_empty() {
            summary = seen.get(&m.message_id).map(|e| e.summary.clone()).unwrap_or_default();
        }
        // the user's correction stays where they put it (the inbox)
        let target = if fixed { "keep".to_string() } else { a.handling.get(&category).cloned().unwrap_or_else(|| "keep".into()) };
        groups.entry(target.clone()).or_default().push(*uid);
        entries.push((target, Entry { at: now(), account: a.name.clone(), uid: *uid, message_id: m.message_id.clone(), from_name: m.from_name.clone(), from_addr: m.from_addr.clone(), subject: crate::text::truncate_chars(&m.subject, 200), category, summary, reason, unread: m.unread, backlog, user_fixed: fixed, ..Default::default() }));
    }
    let mut folders: BTreeMap<String, String> = BTreeMap::new();
    for (target, group) in &groups {
        let folder = match c.move_to(group, target).await {
            Ok(Some(f)) => f,
            Ok(None) if target == "delete" => "usunieta".into(),
            Ok(None) => inbox.to_string(),
            Err(e) => {
                ctx.log(&format!("[{}] nie udalo sie przeniesc do {target}: {e}", a.name));
                inbox.to_string()
            }
        };
        folders.insert(target.clone(), folder);
    }
    let mut out = Vec::new();
    for (target, mut e) in entries {
        e.folder = folders.get(&target).cloned().unwrap_or_else(|| inbox.to_string());
        let old = if backlog { " (stara)" } else { "" };
        ctx.log(&format!("[{}]{old} {}: {} | {} -> {} ({})", a.name, label(&e.category, ctx.lang()), e.from_addr, crate::text::truncate_chars(&e.subject, 80), e.folder, e.reason));
        out.push(e);
    }
    let announce = a.list("announce");
    let important = out.iter().any(|e| announce.contains(&e.category));
    log::add(&ctx.paths, out, ctx.cfg.f64_or("mail.log_days", 14.0));
    if !backlog && important {
        if let Ok(mut p) = ctx.pending.lock() {
            p.report = true;
        }
    }
    Ok(())
}

pub fn backlog_report(ctx: &Ctx, a: &Account, started: f64) {
    let entries: Vec<Entry> = log::load(&ctx.paths).into_iter().filter(|e| e.backlog && e.account == a.name && e.at >= started).collect();
    let n = entries.len();
    let intro = if ctx.lang() == "pl" {
        format!("Przejrzałam starą pocztę na koncie {}: {n} {}.", a.name, if n == 1 { "wiadomość" } else { "wiadomości" })
    } else {
        format!("I went through the old mail in {}: {n} e-mails.", a.name)
    };
    let phrases = ctx.cfg.strs("wake.phrases");
    let text = if entries.is_empty() { intro } else { report(&entries, &ReportOptions { lang: ctx.lang(), intro: &intro, names: 3, multi_account: false, wake_phrases: &phrases }) };
    ctx.log(&format!("[{}] przeglad starej poczty zakonczony: {text}", a.name));
    ctx.queue_text(text);
}

pub async fn purge(ctx: &Ctx, a: &Account, c: &mut Imap, inbox: &str) -> Result<()> {
    let days = a.n("delete_after_days", 7.0);
    if days <= 0.0 {
        return Ok(());
    }
    let before = chrono::Local::now().date_naive() - chrono::Duration::days(days as i64);
    let cutoff = now() - days * 86400.0;
    let recent: Vec<String> = log::load(&ctx.paths).into_iter().filter(|e| e.account == a.name && e.at > cutoff && !e.message_id.is_empty()).map(|e| e.message_id).collect();
    let mut names: Vec<String> = a.handling.values().filter(|f| *f != "keep" && *f != "delete").cloned().collect();
    names.dedup();
    for name in names {
        let folder = c.server_folder(&name).await;
        if folder == inbox || !c.exists(&folder).await {
            continue;
        }
        c.select(&folder).await?;
        let mut old = c.search(&format!("BEFORE {}", imap_date(before))).await?;
        if !old.is_empty() && !recent.is_empty() {
            let heads = c.fetch(&old, "BODY.PEEK[HEADER.FIELDS (MESSAGE-ID)]").await?;
            let keep: Vec<u32> = heads.iter().filter(|h| h.header().map(|b| recent.contains(&parse(b).message_id)).unwrap_or(false)).filter_map(|h| h.uid).collect();
            old.retain(|u| !keep.contains(u));
        }
        if !old.is_empty() {
            c.flag(&old, "\\Deleted").await?;
            c.expunge(&old).await?;
            ctx.log(&format!("[{}] usunieto z serwera {} wiadomosci starszych niz {days} dni z {folder}", a.name, old.len()));
        }
    }
    c.select(inbox).await?;
    Ok(())
}
