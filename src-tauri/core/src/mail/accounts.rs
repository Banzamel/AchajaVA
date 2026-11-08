//! Mail accounts from config.json: the shared settings of the "mail" section, overridden by each
//! account. mail.enabled switches the whole watcher; "enabled" in an account only that account.

use std::collections::BTreeMap;

use serde_json::{json, Map, Value};

use crate::config::Config;
use crate::error::{AppError, Result};

pub const CATEGORIES: &[&str] = &["important", "normal", "ad", "spam", "threat"];

fn defaults() -> Value {
    json!({
        "enabled": true, "model": "haiku", "folder": "INBOX", "security": "ssl", "verify_tls": true,
        "announce": ["important", "threat"], "digest_at": [], "quiet_hours": "", "important_hint": "",
        "important_senders": [], "ignored_senders": [], "delete_after_days": 7, "log_days": 14,
        "scan_existing": true, "scan_existing_days": 30, "scan_attachments": true,
        "blocked_extensions": ["exe", "scr", "com", "pif", "bat", "cmd", "vbs", "vbe", "js", "jse", "wsf", "wsh", "hta",
            "msi", "msix", "appx", "lnk", "iso", "img", "vhd", "vhdx", "jar", "ps1", "cpl", "reg", "docm", "xlsm", "pptm", "one"],
        "poll_seconds": 120, "signature": ""
    })
}

/// "pl" for Polish, "en" for anything else (the mail texts exist in these two).
pub fn lang(cfg: &Config) -> &'static str {
    if cfg.language() == "pl" {
        "pl"
    } else {
        "en"
    }
}

/// Where each category goes: a folder name, "keep" (stays in the inbox) or "delete".
pub fn default_handling(lang: &str) -> BTreeMap<String, String> {
    let (ad, threat) = if lang == "pl" { ("Achaja/Reklamy", "Achaja/Kwarantanna") } else { ("Achaja/Ads", "Achaja/Quarantine") };
    [("important", "keep"), ("normal", "keep"), ("ad", ad), ("spam", "Achaja/Spam"), ("threat", threat)]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[derive(Debug, Clone)]
pub struct Account {
    pub raw: Map<String, Value>,
    pub name: String,
    pub address: String,
    pub handling: BTreeMap<String, String>,
}

impl Account {
    pub fn s(&self, key: &str) -> String {
        match self.raw.get(key) {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Number(n)) => n.to_string(),
            _ => String::new(),
        }
    }
    pub fn n(&self, key: &str, default: f64) -> f64 {
        self.raw.get(key).and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))).unwrap_or(default)
    }
    pub fn b(&self, key: &str, default: bool) -> bool {
        self.raw.get(key).and_then(Value::as_bool).unwrap_or(default)
    }
    pub fn list(&self, key: &str) -> Vec<String> {
        self.raw.get(key).and_then(Value::as_array).map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default()
    }
    pub fn host(&self) -> String {
        self.s("host")
    }
    pub fn user(&self) -> String {
        self.s("user")
    }
    pub fn folder(&self) -> String {
        let f = self.s("folder");
        if f.is_empty() {
            "INBOX".into()
        } else {
            f
        }
    }
    pub fn security(&self) -> String {
        let s = self.s("security");
        if s.is_empty() {
            "ssl".into()
        } else {
            s
        }
    }
    pub fn port(&self) -> u16 {
        let p = self.n("port", 0.0) as u16;
        if p > 0 {
            p
        } else if self.security() == "ssl" {
            993
        } else {
            143
        }
    }
    pub fn enabled(&self) -> bool {
        self.b("enabled", true)
    }
    /// A file-name safe form of the name (the account's state file).
    pub fn safe_name(&self) -> String {
        self.name.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect()
    }
}

pub fn accounts(cfg: &Config, include_disabled: bool) -> Vec<Account> {
    let mail = cfg.section("mail");
    let mut base = defaults().as_object().cloned().unwrap_or_default();
    if let Some(m) = mail.as_object() {
        for (k, v) in m {
            if k != "accounts" && k != "enabled" && !k.starts_with('_') {
                base.insert(k.clone(), v.clone());
            }
        }
    }
    let lang = lang(cfg);
    let mut out = Vec::new();
    for raw in mail["accounts"].as_array().cloned().unwrap_or_default() {
        let Some(obj) = raw.as_object() else { continue };
        let mut acct = base.clone();
        for (k, v) in obj {
            if !k.starts_with('_') {
                acct.insert(k.clone(), v.clone());
            }
        }
        let mut handling = default_handling(lang);
        for source in [&base.get("handling").cloned().unwrap_or(Value::Null), &raw["handling"]] {
            if let Some(h) = source.as_object() {
                for (k, v) in h {
                    if let Some(v) = v.as_str() {
                        handling.insert(k.clone(), v.to_string());
                    }
                }
            }
        }
        let a = Account { raw: acct, name: String::new(), address: String::new(), handling };
        let user = a.user();
        let name = if a.s("name").is_empty() { user.clone() } else { a.s("name") };
        let address = if a.s("address").is_empty() { user.clone() } else { a.s("address") };
        if a.host().is_empty() || user.is_empty() {
            continue;
        }
        let a = Account { name, address, ..a };
        if a.enabled() || include_disabled {
            out.push(a);
        }
    }
    out
}

pub fn find_account(cfg: &Config, name: &str) -> Result<Account> {
    accounts(cfg, true)
        .into_iter()
        .find(|a| a.name.eq_ignore_ascii_case(name) || a.address.eq_ignore_ascii_case(name))
        .ok_or_else(|| AppError::msg(format!("Nie ma konta '{name}' w config.json (mail.accounts).")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_shared_settings_and_overrides() {
        let cfg = Config::new(json!({"language": "pl", "mail": {"enabled": true, "delete_after_days": 3, "handling": {"spam": "delete"},
            "accounts": [{"host": "h", "user": "a@b.pl", "delete_after_days": 1}, {"host": "", "user": "x"}, {"host": "h2", "user": "c@d", "enabled": false}]}}));
        let list = accounts(&cfg, false);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "a@b.pl");
        assert_eq!(list[0].n("delete_after_days", 0.0), 1.0);
        assert_eq!(list[0].handling["spam"], "delete");
        assert_eq!(list[0].handling["ad"], "Achaja/Reklamy");
        assert_eq!(list[0].port(), 993);
        assert_eq!(accounts(&cfg, true).len(), 2);
    }
}
