//! The conversation shown in the window (state\chat.jsonl): one JSON line per message, written
//! by the session hooks and background jobs, read incrementally by the window.

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::json::{append_line, now};
use crate::paths::Paths;

const FILE: &str = "chat.jsonl";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatLine {
    pub role: String,
    pub text: String,
    pub at: f64,
}

pub fn append(paths: &Paths, role: &str, text: &str) {
    let text = text.trim();
    if text.is_empty() {
        return;
    }
    let line = json!({ "role": role, "text": text, "at": now() }).to_string();
    let _ = append_line(&paths.state_file(FILE), &line);
}

/// Complete lines after byte `offset`; returns them and the new offset (only whole lines count,
/// a line being written right now waits for the next read). A shorter file starts over.
pub fn read_since(paths: &Paths, offset: u64) -> (Vec<ChatLine>, u64) {
    let Ok(bytes) = std::fs::read(paths.state_file(FILE)) else { return (Vec::new(), 0) };
    let start = if (bytes.len() as u64) < offset { 0 } else { offset as usize };
    let rest = &bytes[start..];
    let end = rest.iter().rposition(|b| *b == b'\n').map(|p| p + 1).unwrap_or(0);
    let lines = String::from_utf8_lossy(&rest[..end])
        .lines()
        .filter_map(|l| serde_json::from_str::<ChatLine>(l).ok())
        .collect();
    (lines, (start + end) as u64)
}

/// Keeps the file short: the newest `keep` lines once it has grown past twice that.
pub fn trim(paths: &Paths, keep: usize) {
    let file = paths.state_file(FILE);
    let Ok(text) = std::fs::read_to_string(&file) else { return };
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() > keep * 2 {
        let _ = std::fs::write(&file, lines[lines.len() - keep..].join("\n") + "\n");
    }
}

/// The last `n` messages (the window's start).
pub fn last(paths: &Paths, n: usize) -> (Vec<ChatLine>, u64) {
    let (all, offset) = read_since(paths, 0);
    let skip = all.len().saturating_sub(n);
    (all.into_iter().skip(skip).collect(), offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_only_whole_new_lines() {
        let dir = tempfile::tempdir().unwrap();
        let p = Paths::at(dir.path(), dir.path());
        append(&p, "user", "  hello ");
        let (lines, off) = read_since(&p, 0);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "hello");
        std::fs::OpenOptions::new()
            .append(true)
            .open(p.state_file(FILE))
            .and_then(|mut f| std::io::Write::write_all(&mut f, b"{\"role\":\"assistant\""))
            .unwrap();
        let (more, off2) = read_since(&p, off);
        assert!(more.is_empty());
        assert_eq!(off2, off);
    }
}
