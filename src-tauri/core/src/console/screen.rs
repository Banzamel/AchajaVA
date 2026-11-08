//! What the hidden Claude Code console shows: recording ("● REC"), the text in the prompt box
//! (also while it is being dictated), the activity line ("✻ Pondering… (12s · esc to interrupt)")
//! and choice dialogs. The texts it looks for can be changed in config.json (wake.screen_text)
//! when Claude Code changes its interface.

use std::sync::OnceLock;

use regex::Regex;
use serde::Serialize;
use serde_json::Value;

use crate::config::Config;

#[derive(Debug, Clone, PartialEq)]
pub struct Markers {
    pub recording: Vec<String>,
    pub dialog: Vec<String>,
    pub busy: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct Screen {
    pub lines: Vec<String>,
    pub rec: bool,
    pub input: String,
    pub activity: String,
    pub busy: bool,
    pub dialog: bool,
}

fn strings(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

pub fn markers(cfg: &Config) -> Markers {
    let mut m = Markers {
        recording: strings(&["● REC", "REC · "]),
        dialog: strings(&["Enter to confirm", "Esc to cancel", "Do you want to", "❯ 1.", "Do you trust"]),
        busy: strings(&["esc to interrupt"]),
    };
    let old = cfg.strs("wake.recording_markers"); // an older name of the setting
    if !old.is_empty() {
        m.recording = old;
    }
    if let Some(Value::Object(custom)) = cfg.get("wake.screen_text") {
        let list = |v: &Value| v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect::<Vec<_>>());
        if let Some(v) = custom.get("recording").and_then(list) {
            m.recording = v;
        }
        if let Some(v) = custom.get("dialog").and_then(list) {
            m.dialog = v;
        }
        if let Some(v) = custom.get("busy").and_then(list) {
            m.busy = v;
        }
    }
    m
}

fn re(cell: &'static OnceLock<Option<Regex>>, pattern: &str) -> Option<&'static Regex> {
    cell.get_or_init(|| Regex::new(pattern).ok()).as_ref()
}

fn tail<'a>(v: &'a [&'a str], n: usize) -> &'a [&'a str] {
    &v[v.len().saturating_sub(n)..]
}

fn any_in(lines: &[&str], marks: &[String]) -> bool {
    lines.iter().any(|l| marks.iter().any(|m| l.contains(m.as_str())))
}

pub fn parse(lines: &[String], m: &Markers) -> Screen {
    static SEP: OnceLock<Option<Regex>> = OnceLock::new();
    static SPIN: OnceLock<Option<Regex>> = OnceLock::new();
    static PROMPT: OnceLock<Option<Regex>> = OnceLock::new();
    static NOISE: OnceLock<Option<Regex>> = OnceLock::new();
    let mut body: Vec<&str> = lines.iter().map(String::as_str).collect();
    while body.last().is_some_and(|l| l.trim().is_empty()) {
        body.pop();
    }
    let nonempty: Vec<&str> = body.iter().copied().filter(|l| !l.trim().is_empty()).collect();
    let rec = any_in(tail(&nonempty, 6), &m.recording);
    let sep = re(&SEP, r"^\s*─{8,}");
    let seps: Vec<usize> = body.iter().enumerate().filter(|(_, l)| sep.is_some_and(|r| r.is_match(l))).map(|(i, _)| i).collect();
    let mut input = String::new();
    let pair = (seps.len() >= 2).then(|| (seps[seps.len() - 2], seps[seps.len() - 1]));
    if let Some((a, b)) = pair.filter(|(a, b)| b - a <= 12) {
        // the prompt box lies between the last two horizontal lines
        let joined = body[a + 1..b].iter().map(|l| l.trim()).collect::<Vec<_>>().join(" ");
        let joined = re(&PROMPT, r"^[>❯]\s?").map_or(joined.clone(), |r| r.replace(&joined, "").into_owned());
        let joined = re(&NOISE, r"[▀-▟\x00-\x1f]").map_or(joined.clone(), |r| r.replace_all(&joined, "").into_owned());
        input = joined.trim().to_string();
        if input.starts_with("Try \"") {
            input.clear(); // the hint of an empty box
        }
    }
    let above: &[&str] = match pair {
        Some((a, _)) => &body[..a],
        None => &body,
    };
    let spin = re(&SPIN, r"^\s*[^\w\s]\s+\w[^()]*…");
    let mut activity = tail(above, 12)
        .iter()
        .rev()
        .find(|l| spin.is_some_and(|r| r.is_match(l)))
        .map(|l| l.trim().to_string())
        .unwrap_or_default();
    for mark in &m.busy {
        if let Ok(r) = Regex::new(&format!(r"\s*·?\s*{}", regex::escape(mark))) {
            activity = r.replace_all(&activity, "").into_owned();
        }
    }
    let activity = activity.replace("()", "").trim_matches([' ', '·']).to_string();
    let busy = !activity.is_empty() || any_in(tail(&nonempty, 8), &m.busy);
    let dialog = !rec && any_in(tail(&nonempty, 16), &m.dialog);
    Screen { lines: body.iter().map(|s| s.to_string()).collect(), rec, input, activity, busy, dialog }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults() -> Markers {
        markers(&Config::new(serde_json::json!({})))
    }

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_string).collect()
    }

    #[test]
    fn reads_dictated_text_and_recording() {
        let s = parse(&lines("history\n──────────────────\n> otwórz kalkulator ▄▆\n──────────────────\n  ● REC · tap to send\n"), &defaults());
        assert!(s.rec);
        assert_eq!(s.input, "otwórz kalkulator");
        assert!(!s.dialog);
    }

    #[test]
    fn reads_activity_and_dialog() {
        let s = parse(&lines("✻ Pondering… (12s · esc to interrupt)\n──────────\n> \n──────────\n"), &defaults());
        assert_eq!(s.activity, "✻ Pondering… (12s)");
        assert!(s.busy);
        let d = parse(&lines("Do you want to proceed?\n❯ 1. Yes\n  2. No\nEsc to cancel\n"), &defaults());
        assert!(d.dialog);
    }

    #[test]
    fn empty_box_hint_is_no_input() {
        let s = parse(&lines("──────────\n> Try \"fix lint errors\"\n──────────\n"), &defaults());
        assert_eq!(s.input, "");
    }
}
