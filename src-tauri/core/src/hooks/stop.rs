//! Stop hook: the [GŁOS] ([VOICE]) section of Achaja's answer is read aloud. Without one, the
//! Achaja session's turn is stopped once and Claude is asked to add it; if it is still missing,
//! the first sentences are read. An empty answer gets a spoken message. A question in the voice
//! part switches on conversation mode (the listener records the reply after speaking). The
//! gate's approval ends with the turn.

use std::sync::OnceLock;

use regex::{Regex, RegexBuilder};
use serde_json::{json, Value};

use crate::chat;
use crate::config;
use crate::paths::Paths;
use crate::state::{self, APPROVAL, FOLLOWUP};
use crate::voice::speak::say;

fn marker() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| RegexBuilder::new(r"^\s*\[(?:G[ŁL]OS|VOICE)\]\s*:?\s*").case_insensitive(true).multi_line(true).build().ok())
        .as_ref()
}

pub fn has_marker(message: &str) -> bool {
    marker().is_some_and(|r| r.is_match(message))
}

/// The text after the last marker, or the first two sentences when there is none.
pub fn voice_text(message: &str) -> String {
    if let Some(last) = marker().and_then(|r| r.find_iter(message).last()) {
        return message[last.end()..].trim().to_string();
    }
    let mut out = Vec::new();
    let mut start = 0;
    let trimmed = message.trim();
    for (i, c) in trimmed.char_indices() {
        if matches!(c, '.' | '!' | '?') && trimmed[i + c.len_utf8()..].starts_with(char::is_whitespace) {
            out.push(trimmed[start..i + c.len_utf8()].trim().to_string());
            start = i + c.len_utf8();
            if out.len() == 2 {
                break;
            }
        }
    }
    if out.len() < 2 && start < trimmed.len() {
        out.push(trimmed[start..].trim().to_string());
    }
    out.into_iter().filter(|s| !s.is_empty()).take(2).collect::<Vec<_>>().join(" ")
}

fn no_voice_reason(lang: &str) -> &'static str {
    if lang == "en" {
        "The reply has no [VOICE] section and the user is not looking at the screen. Add one [VOICE] line at the very end now with a short summary to be read aloud (do not repeat the whole answer)."
    } else {
        "Odpowiedz nie ma sekcji [GŁOS], a uzytkownik nie patrzy na ekran. Dopisz teraz na samym koncu jedna linie [GŁOS] z krotkim podsumowaniem do przeczytania na glos (bez powtarzania calej odpowiedzi)."
    }
}

/// The JSON to print (Some when the turn must not end yet).
pub fn handle(paths: &Paths, data: &Value, session: bool, job: bool) -> Option<Value> {
    if job {
        return None; // a background job: its runner speaks the result when nobody records
    }
    let cfg = config::load_or_default(paths);
    let message = data["last_assistant_message"].as_str().unwrap_or("");
    let stop_active = data["stop_hook_active"].as_bool().unwrap_or(false);
    if session && !message.trim().is_empty() && !has_marker(message) && !stop_active {
        return Some(json!({ "decision": "block", "reason": no_voice_reason(cfg.lang2()) }));
    }
    if session {
        state::set_work(paths, "idle", None);
        state::clear(paths, APPROVAL); // the approval lasts one turn
        chat::append(paths, "assistant", message);
        crate::tokens::transcript::schedule(paths, data, &cfg.str_or("achaja.effort", ""), !message.trim().is_empty());
    }
    let mut text = voice_text(message);
    if text.is_empty() && session {
        text = if cfg.lang2() == "en" { "I didn't get an answer. Please say that again." } else { "Nie dostałam odpowiedzi. Powtórz, proszę, polecenie." }.to_string();
    }
    if !text.is_empty() {
        state::set_last_voice(paths, &text);
        if text.contains('?') && session {
            state::stamp(paths, FOLLOWUP);
        }
        say(paths, &text);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn takes_the_last_voice_section() {
        assert_eq!(voice_text("Długi tekst.\n[GŁOS] Gotowe.\n"), "Gotowe.");
        assert_eq!(voice_text("a\n[VOICE]: Done?"), "Done?");
        assert_eq!(voice_text("Pierwsze zdanie. Drugie! Trzecie."), "Pierwsze zdanie. Drugie!");
        assert!(!has_marker("bez znacznika"));
    }
}
