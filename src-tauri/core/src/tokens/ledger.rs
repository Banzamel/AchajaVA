//! state\tokens.jsonl: one line per voice command of the Achaja session, background job run
//! and project agent run. Tokens are stored per model and priced only when read, so a price
//! changed in config.json applies to the past too. The file is trimmed to the last 120 days
//! once it grows past 4 MB.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::Tokens;
use crate::json::{append_line, now, write_text};
use crate::paths::Paths;

pub const FILE: &str = "tokens.jsonl";
const MAX_BYTES: u64 = 4 * 1024 * 1024;
const KEEP_DAYS: f64 = 120.0;

/// "session" (a voice command), "job" or "agent".
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Entry {
    pub at: f64,
    pub kind: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub session: String,
    #[serde(default)]
    pub models: BTreeMap<String, Tokens>,
    #[serde(default)]
    pub ms: u64,
    #[serde(default)]
    pub turns: u64,
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub effort: String,
    #[serde(default)]
    pub retries: u64,
}

impl Entry {
    pub fn tokens(&self) -> Tokens {
        let mut t = Tokens::default();
        self.models.values().for_each(|m| t.add(m));
        t
    }

    /// The model that wrote the most (the one a run is named by in the statistics).
    pub fn main_model(&self) -> &str {
        self.models.iter().max_by_key(|(_, t)| t.output).map(|(m, _)| m.as_str()).unwrap_or("")
    }

    /// From the final result of `claude -p --output-format json|stream-json`.
    pub fn from_result(kind: &str, name: &str, result: &Value, effort: &str) -> Entry {
        let mut models = BTreeMap::new();
        if let Some(map) = result["modelUsage"].as_object() {
            for (model, u) in map {
                models.insert(model.clone(), Tokens::from_model_usage(u));
            }
        }
        if models.is_empty() && result["usage"].is_object() {
            models.insert("unknown".into(), Tokens::from_usage(&result["usage"]));
        }
        Entry {
            at: now(),
            kind: kind.into(),
            name: name.into(),
            session: result["session_id"].as_str().unwrap_or("").into(),
            models,
            ms: result["duration_ms"].as_u64().unwrap_or(0),
            turns: result["num_turns"].as_u64().unwrap_or(0),
            ok: !result["is_error"].as_bool().unwrap_or(true),
            effort: effort.into(),
            retries: 0,
        }
    }
}

pub fn append(paths: &Paths, entry: &Entry) {
    let file = paths.state_file(FILE);
    if let Ok(line) = serde_json::to_string(entry) {
        let _ = append_line(&file, &line);
    }
    if std::fs::metadata(&file).map(|m| m.len()).unwrap_or(0) > MAX_BYTES {
        let keep: Vec<String> = read(paths, now() - KEEP_DAYS * 86400.0).iter().filter_map(|e| serde_json::to_string(e).ok()).collect();
        let _ = write_text(&file, &(keep.join("\n") + "\n"));
    }
}

/// Entries from `since` on (a broken line is skipped).
pub fn read(paths: &Paths, since: f64) -> Vec<Entry> {
    let text = crate::json::read_text(&paths.state_file(FILE)).unwrap_or_default();
    text.lines().filter_map(|l| serde_json::from_str::<Entry>(l).ok()).filter(|e| e.at >= since).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_from_a_result_and_round_trips() {
        let dir = std::env::temp_dir().join(format!("achaja-ledger-{}", std::process::id()));
        let paths = Paths::at(&dir, &dir);
        let r = json!({"session_id": "s1", "duration_ms": 754, "num_turns": 2, "is_error": false,
            "modelUsage": {"claude-haiku-5-5": {"inputTokens": 2, "outputTokens": 4}, "claude-opus-5-5": {"outputTokens": 9}}});
        let e = Entry::from_result("job", "poczta", &r, "low");
        assert!(e.ok && e.turns == 2 && e.ms == 754);
        assert_eq!(e.main_model(), "claude-opus-5-5");
        assert_eq!(e.tokens().total(), 15);
        append(&paths, &e);
        assert_eq!(read(&paths, 0.0), vec![e]);
        assert!(read(&paths, now() + 10.0).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
