//! The tokens of one voice command, read from the Achaja session's transcript (the jsonl file
//! Claude Code writes in ~/.claude/projects) when the Stop hook ends the turn. Claude Code writes
//! one answer in several lines with the same message id and the same usage, so lines are counted
//! once per id; only lines since the command was given count (a resumed session's transcript
//! holds the earlier conversation too). Reading starts where the last turn stopped.

use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use serde_json::{json, Value};

use super::ledger::{self, Entry};
use super::Tokens;
use crate::json::{now, num};
use crate::paths::Paths;
use crate::state;

pub const TURN: &str = "turn.json"; // {at}: when the current voice command was given
const CURSOR: &str = "tokens_cursor.json"; // {path, offset}: how far the transcript was read

fn timestamp(line: &Value) -> f64 {
    line["timestamp"].as_str().and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok()).map(|t| t.timestamp_millis() as f64 / 1000.0).unwrap_or(0.0)
}

/// Tokens per model of the assistant lines written at or after `since`.
pub fn parse(text: &str, since: f64) -> BTreeMap<String, Tokens> {
    let mut by_id: BTreeMap<String, (String, Tokens)> = BTreeMap::new();
    for (i, line) in text.lines().enumerate() {
        let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
        let msg = &v["message"];
        if v["type"] != "assistant" || !msg["usage"].is_object() || timestamp(&v) < since {
            continue;
        }
        let model = msg["model"].as_str().unwrap_or("unknown");
        if model.starts_with('<') {
            continue; // "<synthetic>": a message Claude Code made up locally, no model call
        }
        let id = msg["id"].as_str().map(str::to_string).unwrap_or_else(|| format!("line{i}"));
        by_id.insert(id, (model.to_string(), Tokens::from_usage(&msg["usage"])));
    }
    let mut out: BTreeMap<String, Tokens> = BTreeMap::new();
    for (model, t) in by_id.into_values() {
        out.entry(model).or_default().add(&t);
    }
    out
}

/// The complete lines from `offset` on, and the offset after the last complete line.
fn read_from(path: &Path, offset: u64) -> Option<(String, u64)> {
    let mut f = std::fs::File::open(path).ok()?;
    let len = f.metadata().ok()?.len();
    let start = if offset > len { 0 } else { offset };
    f.seek(SeekFrom::Start(start)).ok()?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).ok()?;
    let end = buf.iter().rposition(|b| *b == b'\n').map(|i| i + 1).unwrap_or(0);
    Some((String::from_utf8_lossy(&buf[..end]).into_owned(), start + end as u64))
}

/// Called by the Stop hook of the Achaja session: one ledger line for the command just answered.
pub fn record_turn(paths: &Paths, data: &Value, effort: &str, ok: bool) {
    let Some(path) = data["transcript_path"].as_str().filter(|p| !p.is_empty()) else { return };
    let cursor = state::read(paths, CURSOR);
    let offset = if cursor["path"].as_str() == Some(path) { num(&cursor, "offset") as u64 } else { 0 };
    let started = state::at(paths, TURN);
    let Some((text, end)) = read_from(Path::new(path), offset) else { return };
    state::write(paths, CURSOR, &json!({ "path": path, "offset": end }));
    // a turn without its start (the hook missed it) counts only the last ten minutes
    let since = if started > 0.0 { started - 1.0 } else { now() - 600.0 };
    let models = parse(&text, since);
    if models.is_empty() {
        return;
    }
    let ms = if started > 0.0 { ((now() - started) * 1000.0) as u64 } else { 0 };
    let session = data["session_id"].as_str().unwrap_or("").to_string();
    ledger::append(paths, &Entry { at: now(), kind: "session".into(), session, models, ms, turns: 1, ok, effort: effort.into(), ..Default::default() });
    state::clear(paths, TURN);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(id: &str, ts: &str, model: &str, out: u64) -> String {
        json!({"type": "assistant", "timestamp": ts, "message": {"id": id, "model": model, "usage": {"input_tokens": 1, "output_tokens": out}}}).to_string()
    }

    #[test]
    fn counts_each_message_once_since_the_command() {
        let text = [
            line("old", "2026-11-14T10:00:00Z", "claude-sonnet-5-5", 100),
            line("a", "2026-11-14T10:05:00Z", "claude-sonnet-5-5", 10),
            line("a", "2026-11-14T10:05:01Z", "claude-sonnet-5-5", 10),
            line("b", "2026-11-14T10:05:02Z", "claude-haiku-5-5", 3),
            line("c", "2026-11-14T10:05:03Z", "<synthetic>", 0),
            "{\"type\":\"user\"}".to_string(),
            "not json".to_string(),
        ]
        .join("\n");
        let since = chrono::DateTime::parse_from_rfc3339("2026-11-14T10:04:00Z").map(|t| t.timestamp() as f64).unwrap_or(0.0);
        let m = parse(&text, since);
        assert_eq!(m.len(), 2);
        assert_eq!(m["claude-sonnet-5-5"], Tokens { input: 1, output: 10, ..Default::default() });
        assert_eq!(m["claude-haiku-5-5"].output, 3);
    }

    #[test]
    fn reads_only_complete_lines_and_records() {
        let dir = std::env::temp_dir().join(format!("achaja-transcript-{}", std::process::id()));
        let paths = Paths::at(&dir, &dir);
        std::fs::create_dir_all(paths.state()).ok();
        let file = dir.join("t.jsonl");
        let ts = chrono::Utc::now().to_rfc3339();
        std::fs::write(&file, format!("{}\n{{\"half", line("a", &ts, "claude-opus-5-5", 5))).ok();
        state::write(&paths, TURN, &json!({"at": now() - 5.0}));
        record_turn(&paths, &json!({"transcript_path": file.to_string_lossy(), "session_id": "s"}), "low", true);
        let e = ledger::read(&paths, 0.0);
        assert_eq!(e.len(), 1);
        assert!(e[0].ms >= 4000 && e[0].kind == "session" && e[0].effort == "low");
        let offset = num(&state::read(&paths, CURSOR), "offset") as usize;
        assert_eq!(offset, line("a", &ts, "claude-opus-5-5", 5).len() + 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
