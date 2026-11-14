//! The mail accounts from the settings, for phone alerts while the mail watcher is off: new
//! messages of each inbox since the last check are read without marking them read (BODY.PEEK),
//! sorted by the same model call as the watcher's (no tools), and the important ones become
//! alerts. Nothing is moved or changed on the server. The first check of an account only notes
//! where its inbox ends, so old mail never floods the phone.

use serde_json::{json, Value};

use crate::config::Config;
use crate::error::Result;
use crate::mail::accounts::{accounts, Account};
use crate::mail::imap::connect;
use crate::mail::neutral::speak_name;
use crate::mail::parse::{parse, Mail};
use crate::mail::watch::classify::{classify, ModelVerdict};

const MAX_PER_CHECK: usize = 20;

/// The model call blocks; inside the phone loop's multi-thread runtime it must say so.
fn sort(mails: &[(String, Mail)], a: &Account, lang: &str) -> std::collections::HashMap<String, ModelVerdict> {
    let run = || classify(mails, a, lang, "phone", &|_| {});
    match tokio::runtime::Handle::try_current() {
        Ok(h) if h.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread => tokio::task::block_in_place(run),
        _ => run(),
    }
}

/// The alert of one message the model found important.
pub fn item(m: &Mail, v: &ModelVerdict) -> Option<Value> {
    (v.category == "important").then(|| json!({"sender": speak_name(&m.from_name, &m.from_addr), "matter": if v.summary.is_empty() { &m.subject } else { &v.summary }}))
}

async fn check_account(a: &Account, lang: &str, st: &mut Value) -> Result<Vec<Value>> {
    let mut c = connect(a).await?;
    let inbox = c.server_folder(&a.folder()).await;
    let mailbox = c.select(&inbox).await?;
    let validity = mailbox.uid_validity.map(u64::from);
    if st["uidvalidity"].as_u64() != validity {
        let last = c.search("ALL").await?.into_iter().max().unwrap_or(0);
        *st = json!({"uidvalidity": validity, "last_uid": last});
        c.logout().await;
        return Ok(Vec::new());
    }
    let last = st["last_uid"].as_u64().unwrap_or(0) as u32;
    let mut new: Vec<u32> = c.search(&format!("UID {}:*", last + 1)).await?.into_iter().filter(|u| *u > last).collect();
    if let Some(max) = new.iter().max() {
        st["last_uid"] = json!(max);
    }
    new.reverse();
    new.truncate(MAX_PER_CHECK); // the newest ones after a long pause
    let fetched = c.fetch(&new, "BODY.PEEK[]").await?;
    c.logout().await;
    let mails: Vec<(String, Mail)> = fetched.iter().filter_map(|f| Some((f.uid?.to_string(), parse(f.body()?)))).collect();
    if mails.is_empty() {
        return Ok(Vec::new());
    }
    let verdicts = sort(&mails, a, lang);
    Ok(mails.iter().filter_map(|(id, m)| verdicts.get(id).and_then(|v| item(m, v))).collect())
}

/// Alerts of all enabled accounts; `state` keeps each account's last UID. Errors go to `log`.
pub async fn check(cfg: &Config, lang: &str, state: &mut Value, log: &dyn Fn(&str)) -> Vec<Value> {
    let mut out = Vec::new();
    if !state.is_object() {
        *state = json!({});
    }
    for a in accounts(cfg, false) {
        let mut st = state[&a.name].clone();
        match check_account(&a, lang, &mut st).await {
            Ok(items) => out.extend(items),
            Err(e) => log(&format!("[{}] {}", a.name, crate::text::truncate_chars(&e.to_string(), 200))),
        }
        state[&a.name] = st;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_important_mail_becomes_an_alert() {
        let m = Mail { from_name: "Kasia".into(), from_addr: "k@x.pl".into(), subject: "Spotkanie".into(), ..Default::default() };
        let v = |c: &str, s: &str| ModelVerdict { category: c.into(), summary: s.into(), reason: String::new() };
        assert_eq!(item(&m, &v("important", "")), Some(json!({"sender": "Kasia", "matter": "Spotkanie"})));
        assert_eq!(item(&m, &v("important", "termin jutro")).map(|i| i["matter"].clone()), Some(json!("termin jutro")));
        assert_eq!(item(&m, &v("ad", "x")), None);
    }
}
