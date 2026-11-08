//! The mail log (state\mail\messages.json): only metadata (sender, subject, category, folder,
//! the model's one-line summary) of the last log_days days, never message content.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{AppError, Result};
use crate::json::{now, read_json, write_json};
use crate::paths::Paths;

const LIMIT: usize = 3000;
pub const USER_FIX: &str = "poprawione przez uzytkownika";
static LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Entry {
    #[serde(default)]
    pub id: u64,
    pub at: f64,
    pub account: String,
    #[serde(default)]
    pub uid: u32,
    #[serde(default)]
    pub message_id: String,
    #[serde(default)]
    pub from_name: String,
    #[serde(default)]
    pub from_addr: String,
    #[serde(default)]
    pub subject: String,
    pub category: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub folder: String,
    #[serde(default = "yes")]
    pub unread: bool,
    #[serde(default)]
    pub backlog: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub user_fixed: bool,
    /// The account's handling when the entry was made (to find the message again).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handling: Option<HashMap<String, String>>,
}

fn yes() -> bool {
    true
}

fn file(paths: &Paths) -> std::path::PathBuf {
    paths.state().join("mail").join("messages.json")
}

pub fn load(paths: &Paths) -> Vec<Entry> {
    let raw: Value = read_json(&file(paths), Value::Null);
    raw.as_array().map(|a| a.iter().filter_map(|e| serde_json::from_value(e.clone()).ok()).collect()).unwrap_or_default()
}

fn prune(mut log: Vec<Entry>, log_days: f64) -> Vec<Entry> {
    let cutoff = now() - log_days * 86400.0;
    log.retain(|e| e.at >= cutoff);
    let skip = log.len().saturating_sub(LIMIT);
    log.into_iter().skip(skip).collect()
}

/// Adds entries with the next free numbers (the numbers `achaja mail list` shows).
pub fn add(paths: &Paths, mut entries: Vec<Entry>, log_days: f64) -> Vec<Entry> {
    let _guard = LOCK.lock();
    let mut log = load(paths);
    let mut next = log.iter().map(|e| e.id).max().unwrap_or(0) + 1;
    for e in entries.iter_mut() {
        e.id = next;
        next += 1;
    }
    log.extend(entries.clone());
    let _ = write_json(&file(paths), &prune(log, log_days));
    entries
}

pub fn update(paths: &Paths, id: u64, change: impl Fn(&mut Entry)) {
    let _guard = LOCK.lock();
    let mut log = load(paths);
    for e in log.iter_mut().filter(|e| e.id == id) {
        change(e);
    }
    let _ = write_json(&file(paths), &log);
}

pub fn get(paths: &Paths, id: u64) -> Result<Entry> {
    load(paths).into_iter().find(|e| e.id == id).ok_or_else(|| AppError::msg(format!("Nie ma wiadomosci o numerze {id} (achaja.cmd mail list pokazuje numery).")))
}

/// The newest entry per Message-ID of an account (mail that comes back to the inbox).
pub fn last_by_message_id(paths: &Paths, account: &str) -> HashMap<String, Entry> {
    let mut map = HashMap::new();
    for e in load(paths) {
        if e.account == account && !e.message_id.is_empty() {
            map.insert(e.message_id.clone(), e);
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_entries_and_reads_old_files() {
        let dir = tempfile::tempdir().unwrap();
        let p = Paths::at(dir.path(), dir.path());
        let e = Entry { at: now(), account: "a".into(), category: "ad".into(), message_id: "<m>".into(), ..Default::default() };
        let added = add(&p, vec![e.clone(), e], 14.0);
        assert_eq!(added.iter().map(|e| e.id).collect::<Vec<_>>(), [1, 2]);
        update(&p, 2, |e| e.category = "normal".into());
        assert_eq!(get(&p, 2).unwrap().category, "normal");
        assert_eq!(last_by_message_id(&p, "a")["<m>"].id, 2);
    }
}
