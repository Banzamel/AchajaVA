//! Speaking the mail report: when something to announce arrived or at the digest times, but
//! never in quiet hours and never while the user talks to Achaja (speech, recording, an awaited
//! answer, work in progress).

use std::sync::Arc;
use std::time::Duration;

use chrono::{NaiveTime, Timelike};
use serde_json::{json, Value};

use super::Ctx;
use crate::json::{now, read_json, write_json};
use crate::mail::log;
use crate::mail::report::{report, ReportOptions};
use crate::state::user_busy;

fn clock(s: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(s.trim(), "%H:%M").ok()
}

/// "22:00-07:00": inside the range (across midnight too); an empty or broken spec never is.
pub fn inside(spec: &str, now: NaiveTime) -> bool {
    let Some((a, b)) = spec.split_once('-') else { return false };
    let (Some(start), Some(end)) = (clock(a), clock(b)) else { return false };
    if start <= end {
        start <= now && now < end
    } else {
        now >= start || now < end
    }
}

/// A digest time passed today that was not reported yet (each time once a day).
fn digest_due(times: &[String], state: &mut Value) -> bool {
    let today = chrono::Local::now().date_naive().to_string();
    let now = chrono::Local::now().time().with_nanosecond(0).unwrap_or_default();
    for t in times {
        if clock(t).is_some_and(|c| now >= c) && state["digests"][t].as_str() != Some(today.as_str()) {
            if !state["digests"].is_object() {
                state["digests"] = json!({});
            }
            state["digests"][t] = json!(today);
            return true;
        }
    }
    false
}

pub async fn run(ctx: Arc<Ctx>, multi: bool) {
    let file = ctx.paths.state().join("mail").join("report.json");
    let mut state: Value = read_json(&file, json!({}));
    if state["last"].as_f64().is_none() {
        state["last"] = json!(now());
    }
    let _ = write_json(&file, &state);
    let mut digest_pending = false;
    loop {
        tokio::time::sleep(Duration::from_secs(3)).await;
        if digest_due(&ctx.cfg.strs("mail.digest_at"), &mut state) {
            digest_pending = true;
            let _ = write_json(&file, &state);
        }
        let wanted = ctx.pending.lock().map(|p| p.report || !p.texts.is_empty()).unwrap_or(false) || digest_pending;
        let quiet = inside(&ctx.cfg.str_or("mail.quiet_hours", ""), chrono::Local::now().time());
        if !wanted || quiet || user_busy(&ctx.paths) {
            continue;
        }
        // a short pause, in case the user is just starting to speak
        tokio::time::sleep(Duration::from_millis(1000 + rand::random::<u64>() % 1000)).await;
        if user_busy(&ctx.paths) {
            continue;
        }
        let (mut texts, want_report) = match ctx.pending.lock() {
            Ok(mut p) => {
                let out = (std::mem::take(&mut p.texts), p.report || digest_pending);
                p.report = false;
                out
            }
            Err(_) => continue,
        };
        digest_pending = false;
        if want_report {
            let last = state["last"].as_f64().unwrap_or(0.0);
            let entries: Vec<_> = log::load(&ctx.paths).into_iter().filter(|e| e.at > last && !e.backlog).collect();
            if !entries.is_empty() {
                let phrases = ctx.cfg.strs("wake.phrases");
                let intro = if ctx.lang() == "pl" { "Poczta." } else { "Mail." };
                texts.insert(0, report(&entries, &ReportOptions { lang: ctx.lang(), intro, names: 3, multi_account: multi, wake_phrases: &phrases }));
            }
            state["last"] = json!(now());
            let _ = write_json(&file, &state);
        }
        if !texts.is_empty() {
            let text = texts.join(" ");
            crate::state::set_last_voice(&ctx.paths, &text);
            crate::voice::speak::say(&ctx.paths, &text);
            ctx.log(&format!("Powiedziano: {}", crate::text::truncate_chars(&text, 400)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_hours_across_midnight() {
        let t = |s| NaiveTime::parse_from_str(s, "%H:%M").unwrap();
        assert!(inside("22:00-07:00", t("23:30")));
        assert!(inside("22:00-07:00", t("06:59")));
        assert!(!inside("22:00-07:00", t("12:00")));
        assert!(inside("08:00-22:00", t("12:00")));
        assert!(!inside("", t("12:00")));
    }
}
