//! The phone alerts at a glance for the window's "Today" panel: which sources are on, the
//! connectors' last known status and the last check - read from config.json and
//! state\phone\state.json only, never a network or model call.

use serde::Serialize;
use serde_json::{json, Value};

use crate::config::Config;
use crate::json::read_json;
use crate::paths::Paths;

#[derive(Debug, Serialize, PartialEq)]
pub struct Overview {
    pub enabled: bool,
    pub gmail: bool,
    pub gmail_status: String,
    pub accounts: usize,
    pub imap: bool,
    pub calendar: bool,
    pub calendar_status: String,
    pub last_check: f64,
    pub pending: usize,
}

pub fn from(cfg: &Config, st: &Value) -> Overview {
    let s = super::settings(cfg);
    Overview {
        enabled: s.enabled,
        gmail: s.gmail,
        gmail_status: st["connector"].as_str().unwrap_or("").to_string(),
        accounts: crate::mail::accounts::accounts(cfg, false).len(),
        imap: s.imap,
        calendar: s.calendar,
        calendar_status: st["calendar_connector"].as_str().unwrap_or("").to_string(),
        last_check: st["last_check"].as_f64().unwrap_or(0.0),
        pending: st["pending"].as_array().map_or(0, Vec::len),
    }
}

pub fn overview(paths: &Paths) -> Overview {
    let st: Value = read_json(&paths.state().join("phone").join("state.json"), json!({}));
    from(&crate::config::load_or_default(paths), &st)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_sources_and_last_state() {
        let cfg = Config::new(json!({"phone_alerts": {"enabled": true, "calendar": true}}));
        let o = from(&cfg, &json!({"connector": "connected", "last_check": 12.0, "pending": [1, 2]}));
        assert!(o.enabled && o.gmail && o.imap && o.calendar);
        assert_eq!((o.gmail_status.as_str(), o.calendar_status.as_str(), o.last_check, o.pending, o.accounts), ("connected", "", 12.0, 2, 0));
    }
}
