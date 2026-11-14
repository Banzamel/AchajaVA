//! Important mail and calendar reminders on the phone: a push and a spoken message through the
//! Home Assistant app. Two independent mail sources: Gmail through the claude.ai Gmail connector
//! (a separate `claude -p` with two read-only tools) and the mail accounts from the settings
//! (IMAP, read-only) - either works without the other. Calendar reminders come only through the
//! Google Calendar connector. The text on the phone is built here from the model's JSON fields,
//! with links, addresses and long numbers removed.

mod calendar;
mod check;
pub mod connector;
mod gmail;
mod inbox;
pub mod overview;

use clap::Subcommand;
use serde_json::{json, Value};

use crate::config::Config;
use crate::error::{AppError, Result};
use crate::homeassistant::client::Ha;
use crate::mail::neutral::neutral;
use crate::paths::Paths;

pub use check::{check, run_loop};

#[derive(Debug, Subcommand)]
pub enum PhoneCmd {
    /// Ustawienia, konektor, ostatnie sprawdzenie
    Status,
    /// Proba na telefonie
    Test {
        #[arg(long, default_value = "To próba powiadomienia głosowego.")]
        text: String,
        #[arg(long)]
        voice_only: bool,
        #[arg(long)]
        push_only: bool,
    },
    /// Jedno sprawdzenie teraz
    Once,
    /// Petla w tle (uruchamia ja Achaja)
    Run,
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub enabled: bool,
    pub service: String,
    pub interval: f64,
    pub voice: bool,
    pub push: bool,
    pub voice_hours: String,
    pub model: String,
    pub imap: bool,
    pub media_stream: String,
    /// Gmail through the claude.ai connector.
    pub gmail: bool,
    /// Reminders before Google Calendar events (connector only), calendar_minutes before.
    pub calendar: bool,
    pub calendar_minutes: f64,
}

pub fn settings(cfg: &Config) -> Settings {
    let service = cfg.str_or("phone_alerts.notify_service", "");
    Settings {
        enabled: cfg.bool_or("phone_alerts.enabled", false),
        service: service.strip_prefix("notify.").unwrap_or(&service).to_string(),
        interval: cfg.f64_or("phone_alerts.interval_minutes", 5.0).max(1.0),
        voice: cfg.bool_or("phone_alerts.voice", true),
        push: cfg.bool_or("phone_alerts.push", true),
        voice_hours: cfg.str_or("phone_alerts.voice_hours", "08:00-22:00"),
        model: cfg.str_or("phone_alerts.model", "haiku"),
        imap: cfg.bool_or("phone_alerts.imap", true),
        media_stream: cfg.str_or("phone_alerts.media_stream", "alarm_stream_max"),
        gmail: cfg.bool_or("phone_alerts.gmail", true),
        calendar: cfg.bool_or("phone_alerts.calendar", false),
        calendar_minutes: cfg.f64_or("phone_alerts.calendar_minutes", 15.0).clamp(1.0, 120.0),
    }
}

/// A field from the model for the phone: no links, addresses, long numbers or markup.
pub fn clean(value: &str, limit: usize) -> String {
    let mut t = neutral(value, true);
    for p in [r"(?i)(?:[a-z][a-z0-9+.-]*://|www\.)\S+", r"\S+@\S+", r"(?i)\b[\w-]+\.(?:com|pl|net|org|io|eu|ru|info|xyz|app|link|ly)\b\S*", r"\d[\d \-]{6,}\d", r"[<>\[\]{}`*_#|\\]"] {
        if let Ok(r) = regex::Regex::new(p) {
            t = r.replace_all(&t, "").into_owned();
        }
    }
    let t = crate::text::squash(&t);
    let t = t.trim_matches([' ', ',', ';', ':', '-']);
    crate::text::truncate_chars(t, limit).trim_end_matches([' ', ',', ';', ':', '-']).to_string()
}

/// {title, text, voice} of one alert, or None when there is nothing to say.
pub fn alert_from(item: &Value, lang: &str) -> Option<Value> {
    let sender = clean(item["sender"].as_str().unwrap_or(""), 60);
    let matter = crate::text::cap(&clean(item["matter"].as_str().unwrap_or(""), 140));
    if sender.is_empty() && matter.is_empty() {
        return None;
    }
    let pl = lang == "pl";
    let deadline = clean(item["deadline"].as_str().unwrap_or(""), 50);
    let mut parts = vec![matter];
    if !deadline.is_empty() {
        parts.push(format!("{}: {deadline}", if pl { "Termin" } else { "Deadline" }));
    }
    let body = parts.into_iter().filter(|p| !p.is_empty()).collect::<Vec<_>>().join(". ");
    let intro = if pl { "Ważny mail" } else { "Important mail" };
    let title = if sender.is_empty() { intro.to_string() } else { sender.clone() };
    let voice = format!("{intro}. {sender}. {body}.").replace("..", ".").replace(" .", "");
    Some(json!({"title": title, "text": if body.is_empty() { title.clone() } else { body }, "voice": voice}))
}

/// Sends the push and the spoken message; an error when Home Assistant refused.
pub async fn send(paths: &Paths, alert: &Value, s: &Settings, voice: bool, push: bool) -> Result<()> {
    if !regex::Regex::new(r"^mobile_app_[a-z0-9_]+$")?.is_match(&s.service) {
        return Err(AppError::msg("phone_alerts.notify_service musi miec postac mobile_app_<telefon>."));
    }
    let ha = Ha::new(paths, false)?;
    let tag = format!("achaja-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
    if push && s.push {
        ha.service("notify", &s.service, &json!({"title": alert["title"], "message": alert["text"], "data": {"ttl": 0, "priority": "high", "tag": tag}})).await?;
    }
    if voice && s.voice {
        let mut data = json!({"ttl": 0, "priority": "high", "tts_text": alert["voice"]});
        if !s.media_stream.is_empty() {
            data["media_stream"] = json!(s.media_stream);
        }
        ha.service("notify", &s.service, &json!({"message": "TTS", "data": data})).await?;
    }
    Ok(())
}

pub fn cli(paths: &Paths, cmd: PhoneCmd) -> Result<i32> {
    let cfg = crate::config::load(paths)?;
    let s = settings(&cfg);
    match cmd {
        PhoneCmd::Status => check::print_status(paths, &s),
        PhoneCmd::Test { text, voice_only, push_only } => {
            let alert = json!({"title": "Achaja", "text": text, "voice": text});
            crate::runtime::block_on(send(paths, &alert, &s, !push_only, !voice_only))?;
            println!("Wyslano do Home Assistant.");
        }
        PhoneCmd::Once => {
            crate::runtime::block_on(async { Ok(check(paths, &cfg, &s).await) })?;
            check::print_status(paths, &s);
        }
        PhoneCmd::Run => return Ok(run_loop(paths)),
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alert_without_links_or_addresses() {
        let a = alert_from(&json!({"sender": "Kasia <k@x.pl>", "matter": "zaproszenie na rozmowę, zobacz https://evil.com", "deadline": "piątek 10:00"}), "pl").unwrap();
        assert_eq!(a["title"], "Kasia");
        assert_eq!(a["text"], "Zaproszenie na rozmowę, zobacz. Termin: piątek 10:00");
        assert!(!a["voice"].as_str().unwrap().contains("http"));
    }
}
