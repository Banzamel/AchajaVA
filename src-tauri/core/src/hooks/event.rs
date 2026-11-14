//! UserPromptSubmit and Notification hooks (only in the Achaja session). A prompt: the state
//! becomes "busy", the gate's approval for this turn is written, and a short "I'm on it" is
//! spoken. A notification that Claude waits at the computer: it is said aloud and the state
//! becomes "waiting" (the listener then does not say "still working").

use rand::seq::IndexedRandom;
use serde_json::{json, Value};

use super::yes::is_yes;
use crate::chat;
use crate::config;
use crate::json::now;
use crate::paths::Paths;
use crate::state::{self, APPROVAL};
use crate::voice::speak::say;

/// Without the wake word: the speaker and a desk microphone could wake the listener.
fn notification_text(kind: &str, lang: &str) -> Option<&'static str> {
    match (kind, lang) {
        ("permission_prompt", "en") => Some("I'm waiting for your approval at the computer."),
        ("permission_prompt", _) => Some("Czekam na Twoją zgodę przy komputerze."),
        ("elicitation_dialog", "en") => Some("I asked a question with choices, answer at the computer."),
        ("elicitation_dialog", _) => Some("Zadałam pytanie z wyborem, odpowiedz przy komputerze."),
        _ => None,
    }
}

pub fn handle(paths: &Paths, data: &Value) {
    let cfg = config::load_or_default(paths);
    match data["hook_event_name"].as_str() {
        Some("UserPromptSubmit") => {
            state::set_work(paths, "busy", None);
            state::stamp(paths, crate::tokens::transcript::TURN); // the response time and its tokens
            let prompt = data["prompt"].as_str().unwrap_or("");
            chat::append(paths, "user", prompt);
            // the gate: an approval only from the user's prompt, only for this turn
            let yes = is_yes(prompt, &cfg.strs("wake.end"));
            state::write(paths, APPROVAL, &json!({ "yes": yes, "at": now() }));
            let phrases = cfg.strs("voice.ack_phrases");
            if let Some(p) = phrases.choose(&mut rand::rng()) {
                say(paths, p);
            }
        }
        Some("Notification") => {
            let kind = data["notification_type"].as_str().unwrap_or("");
            if let Some(text) = notification_text(kind, cfg.lang2()) {
                state::set_work(paths, "waiting", Some(kind));
                say(paths, text);
            }
        }
        _ => {}
    }
}
