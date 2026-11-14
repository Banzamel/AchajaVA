//! One round of phone alerts and the loop that runs it every minute. Mail every
//! phone_alerts.interval_minutes: Gmail through the connector (when on and connected) and the mail
//! accounts from the settings (the watcher's log while it runs, otherwise read here, inbox.rs) -
//! each works without the other. Calendar reminders through the Google Calendar connector are
//! checked every minute (calendar.rs). Log: state\phone\phone.log (no mail content), state:
//! state\phone\state.json.

use serde_json::{json, Value};

use super::{alert_from, calendar, gmail, inbox, send, Settings};
use crate::config::Config;
use crate::json::{append_line, now, read_json, write_json};
use crate::paths::Paths;
use crate::process::single_instance;

const MAX_SEEN: usize = 500;
const MAX_ALERTS: usize = 5;
const MAX_REMINDED: usize = 200;

fn dir(paths: &Paths) -> std::path::PathBuf {
    paths.state().join("phone")
}

fn log(paths: &Paths, msg: &str) {
    let file = dir(paths).join("phone.log");
    if std::fs::metadata(&file).map(|m| m.len() > 500_000).unwrap_or(false) {
        let _ = std::fs::rename(&file, file.with_extension("log.1"));
    }
    let _ = append_line(&file, &format!("{} {msg}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")));
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default()
}

/// New important mail of every source as alerts (into st["pending"]).
async fn mail_round(paths: &Paths, cfg: &Config, s: &Settings, st: &mut Value, lang: &str) {
    let started = now();
    let since = st["since"].as_f64().unwrap_or(started - s.interval * 60.0);
    let mut alerts: Vec<Value> = Vec::new();
    if s.gmail {
        let mut seen = strings(&st["seen"]);
        let (statuses, items) = gmail::fetch(paths, s, lang, since, &seen);
        let status = statuses.get("gmail").cloned().unwrap_or_else(|| "brak".into());
        st["connector"] = json!(status);
        match items.filter(|_| status == "connected") {
            Some(items) => {
                let fresh: Vec<&Value> = items.iter().filter(|i| i["id"].as_str().is_some_and(|id| !seen.iter().any(|x| x == id))).collect();
                alerts.extend(fresh.iter().filter(|i| i["notify"] == json!(true)).filter_map(|i| alert_from(i, lang)));
                let mut ids: Vec<String> = fresh.iter().filter_map(|i| i["id"].as_str().map(str::to_string)).collect();
                ids.append(&mut seen);
                ids.truncate(MAX_SEEN);
                st["seen"] = json!(ids);
                log(paths, &format!("Gmail: nowych {}", fresh.len()));
            }
            None if status != "connected" => log(paths, &format!("Konektor Gmail niepolaczony ({status}) - pomijam Gmaila.")),
            None => log(paths, "Gmail: sprawdzenie nieudane - powtorze w nastepnym cyklu."),
        }
    }
    if s.imap {
        if crate::mail::watch_pid(paths).is_some() {
            // the watcher sorts the accounts anyway: its important mail, without a second model call
            let last = st["imap_last"].as_f64().unwrap_or(started);
            for e in crate::mail::log::load(paths).into_iter().filter(|e| e.at > last && e.category == "important" && !e.backlog) {
                let sender = if e.from_name.is_empty() { e.account.clone() } else { e.from_name.clone() };
                let matter = if e.summary.is_empty() { e.subject.clone() } else { e.summary.clone() };
                alerts.extend(alert_from(&json!({"sender": sender, "matter": matter}), lang));
            }
        } else {
            let mut state = st["inbox"].take();
            let items = inbox::check(cfg, lang, &mut state, &|m| log(paths, m)).await;
            alerts.extend(items.iter().filter_map(|i| alert_from(i, lang)));
            st["inbox"] = state;
        }
        st["imap_last"] = json!(started);
    }
    st["since"] = json!(started);
    st["last_check"] = json!(started);
    st["ok"] = json!(true);
    log(paths, &format!("Sprawdzono poczte: waznych {}", alerts.len()));
    push_pending(st, alerts);
}

fn push_pending(st: &mut Value, alerts: Vec<Value>) {
    let mut pending: Vec<Value> = st["pending"].as_array().cloned().unwrap_or_default();
    pending.extend(alerts);
    st["pending"] = json!(pending);
}

/// Reminders of the events starting soon (events re-read every half hour).
fn calendar_round(paths: &Paths, s: &Settings, st: &mut Value, lang: &str) {
    let now_l = chrono::Local::now();
    if now() - st["cal_fetched"].as_f64().unwrap_or(0.0) >= calendar::FETCH_EVERY {
        st["cal_fetched"] = json!(now());
        let (statuses, events) = calendar::fetch(paths, s, now_l);
        let status = statuses.get("calendar").cloned().unwrap_or_else(|| "brak".into());
        st["calendar_connector"] = json!(status);
        match events.filter(|_| status == "connected") {
            Some(ev) => st["cal_events"] = json!(ev),
            None => log(paths, &format!("Kalendarz: brak odczytu (konektor: {status}).")),
        }
    }
    let events: Vec<Value> = st["cal_events"].as_array().cloned().unwrap_or_default();
    let mut reminded = strings(&st["cal_reminded"]);
    let due: Vec<Value> = calendar::due(&events, &reminded, s.calendar_minutes, now_l).into_iter().cloned().collect();
    for e in &due {
        reminded.insert(0, calendar::key(e));
    }
    reminded.truncate(MAX_REMINDED);
    st["cal_reminded"] = json!(reminded);
    push_pending(st, due.iter().map(|e| calendar::alert(e, now_l, lang)).collect());
}

/// Sends the waiting alerts (at most five, then one summary push).
async fn deliver(paths: &Paths, s: &Settings, st: &mut Value, lang: &str) {
    let mut pending: Vec<Value> = st["pending"].as_array().cloned().unwrap_or_default();
    let speak = s.voice_hours.trim().is_empty() || crate::mail::watch::announce::inside(&s.voice_hours, chrono::Local::now().time());
    let (mut sent, mut sent_err) = (0, None);
    while sent < MAX_ALERTS && !pending.is_empty() {
        match send(paths, &pending[0], s, speak, true).await {
            Ok(()) => {
                pending.remove(0);
                sent += 1;
            }
            Err(e) => {
                sent_err = Some(e);
                break;
            }
        }
    }
    if sent_err.is_none() && !pending.is_empty() {
        let n = pending.len();
        let text = if lang == "pl" { format!("Jeszcze {n} ważnych powiadomień.") } else { format!("{n} more important notifications.") };
        if send(paths, &json!({"title": "Achaja", "text": text, "voice": text}), s, false, true).await.is_ok() {
            pending.clear();
        }
    }
    if let Some(e) = sent_err {
        log(paths, &format!("Home Assistant nie przyjal powiadomienia: {}", crate::text::truncate_chars(&e.to_string(), 200)));
    }
    st["pending"] = json!(pending);
}

/// One round: mail when `mail` (or due), the calendar, then sending.
async fn round(paths: &Paths, cfg: &Config, s: &Settings, force_mail: bool) {
    let file = dir(paths).join("state.json");
    let mut st: Value = read_json(&file, json!({}));
    let lang = crate::mail::accounts::lang(cfg);
    if force_mail || now() >= st["last_check"].as_f64().unwrap_or(0.0) + s.interval * 60.0 - 5.0 {
        mail_round(paths, cfg, s, &mut st, lang).await;
    }
    if s.calendar {
        calendar_round(paths, s, &mut st, lang);
    }
    let _ = write_json(&file, &st); // saved before sending: a crash never repeats sent alerts forever
    deliver(paths, s, &mut st, lang).await;
    let _ = write_json(&file, &st);
}

/// `achaja phone once`: everything now.
pub async fn check(paths: &Paths, cfg: &Config, s: &Settings) {
    round(paths, cfg, s, true).await;
}

pub fn run_loop(paths: &Paths) -> i32 {
    if !single_instance("AchajaPhone") {
        return 0;
    }
    let _ = write_json(&dir(paths).join("run.json"), &json!({"pid": std::process::id()}));
    let Ok(rt) = crate::runtime::multi_thread() else { return 1 };
    loop {
        let cfg = crate::config::load_or_default(paths);
        let s = super::settings(&cfg);
        if !s.enabled {
            log(paths, "phone_alerts wylaczone - koncze.");
            return 0;
        }
        rt.block_on(round(paths, &cfg, &s, false));
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}

pub fn print_status(paths: &Paths, s: &Settings) {
    let st: Value = read_json(&dir(paths).join("state.json"), json!({}));
    let hours = if s.voice_hours.is_empty() { "zawsze".to_string() } else { s.voice_hours.clone() };
    let service = if s.service.is_empty() { "(brak)".to_string() } else { s.service.clone() };
    println!("wlaczone: {} | co {} min | telefon: {service} | glos: {} ({hours}) | push: {}", s.enabled, s.interval, s.voice, s.push);
    println!("Gmail (konektor): {} [{}] | konta z ustawien: {} | kalendarz: {} co {} min przed [{}]", s.gmail, st["connector"].as_str().unwrap_or("?"), s.imap, s.calendar, s.calendar_minutes, st["calendar_connector"].as_str().unwrap_or("?"));
    let when = st["last_check"].as_f64().map(|t| crate::plan::schedule::from_ts(t).format("%Y-%m-%d %H:%M").to_string()).unwrap_or_else(|| "nigdy".into());
    println!("ostatnie sprawdzenie poczty: {when} | w kolejce: {}", st["pending"].as_array().map_or(0, Vec::len));
}
