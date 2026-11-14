//! The tokens of one voice command, read from the Achaja session's transcript (the jsonl file
//! Claude Code writes in ~/.claude/projects). The Stop hook runs before Claude Code has written
//! the last answer to that file, so the hook only leaves a note and starts `Achaja.exe hook
//! tokens`, which waits for the turn's closing `turn_duration` line (it also carries the real
//! response time) and then counts. Claude Code writes one answer in several lines with the same
//! message id and the same usage, so lines are counted once per id; only lines since the command
//! was given count (a resumed session's transcript holds the earlier conversation too).

use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::ledger::{self, Entry};
use super::Tokens;
use crate::json::{now, num};
use crate::paths::Paths;
use crate::state;

pub const TURN: &str = "turn.json"; // {at}: when the current voice command was given
const PENDING: &str = "turn_done.json"; // the answered command, waiting to be counted
const CURSOR: &str = "tokens_cursor.json"; // {path, offset}: how far the transcript was read
const WAIT: Duration = Duration::from_secs(30);

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

/// The bytes up to and including the turn's closing line, and its duration in ms; None while
/// the turn is not finished in the file yet.
pub fn turn_end(text: &str, since: f64) -> Option<(usize, u64)> {
    let mut end = 0;
    for line in text.split_inclusive('\n') {
        end += line.len();
        let Ok(v) = serde_json::from_str::<Value>(line.trim()) else { continue };
        if v["type"] == "system" && v["subtype"] == "turn_duration" && timestamp(&v) >= since {
            return Some((end, v["durationMs"].as_u64().unwrap_or(0)));
        }
    }
    None
}

/// The complete lines from `offset` on, and the offset they start at.
fn read_from(path: &Path, offset: u64) -> Option<(String, u64)> {
    let mut f = std::fs::File::open(path).ok()?;
    let len = f.metadata().ok()?.len();
    let start = if offset > len { 0 } else { offset };
    f.seek(SeekFrom::Start(start)).ok()?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).ok()?;
    let end = buf.iter().rposition(|b| *b == b'\n').map(|i| i + 1).unwrap_or(0);
    Some((String::from_utf8_lossy(&buf[..end]).into_owned(), start))
}

/// Stop hook of the Achaja session: notes the answered command and starts the counting process.
pub fn schedule(paths: &Paths, data: &Value, effort: &str, ok: bool) {
    let Some(path) = data["transcript_path"].as_str().filter(|p| !p.is_empty()) else { return };
    let started = state::at(paths, TURN);
    // a turn without its start (the hook missed it) counts only the last ten minutes
    let since = if started > 0.0 { started - 1.0 } else { now() - 600.0 };
    let ms = if started > 0.0 { ((now() - started) * 1000.0) as u64 } else { 0 };
    let session = data["session_id"].as_str().unwrap_or("");
    state::write(paths, PENDING, &json!({ "path": path, "session": session, "since": since, "ms": ms, "effort": effort, "ok": ok }));
    state::clear(paths, TURN);
    let _ = crate::process::spawn_self(&["hook", "tokens"], &paths.data);
}

/// `Achaja.exe hook tokens`: waits for the turn to be in the transcript, then one ledger line.
pub fn record_pending(paths: &Paths) {
    let p = state::read(paths, PENDING);
    let Some(path) = p["path"].as_str().map(str::to_string) else { return };
    let since = num(&p, "since");
    let cursor = state::read(paths, CURSOR);
    let offset = if cursor["path"].as_str() == Some(path.as_str()) { num(&cursor, "offset") as u64 } else { 0 };
    let deadline = Instant::now() + WAIT;
    let (text, start, end, ms) = loop {
        let Some((text, start)) = read_from(Path::new(&path), offset) else { return };
        if let Some((end, ms)) = turn_end(&text, since) {
            break (text, start, end, ms);
        }
        if Instant::now() >= deadline {
            let end = text.len(); // not closed in time: count what is there
            break (text, start, end, 0);
        }
        std::thread::sleep(Duration::from_millis(300));
    };
    state::write(paths, CURSOR, &json!({ "path": path, "offset": start + end as u64 }));
    state::clear(paths, PENDING);
    let models = parse(&text[..end], since);
    if models.is_empty() {
        return;
    }
    let ms = if ms > 0 { ms } else { num(&p, "ms") as u64 };
    let (session, effort) = (p["session"].as_str().unwrap_or("").to_string(), p["effort"].as_str().unwrap_or("").to_string());
    ledger::append(paths, &Entry { at: now(), kind: "session".into(), session, models, ms, turns: 1, ok: p["ok"].as_bool().unwrap_or(true), effort, ..Default::default() });
}

#[cfg(test)]
#[path = "transcript_tests.rs"]
mod tests;
