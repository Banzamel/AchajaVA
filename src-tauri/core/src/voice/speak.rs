//! `achaja speak "text"`: reads a text aloud on the chosen output. Every answer is its own short
//! process; they queue through state\tts.pid (one speaks at a time, none interrupts another),
//! and the listener stops the one speaking with state\tts_stop.json (also on a Cast speaker).

use std::sync::OnceLock;

use regex::Regex;
use serde_json::json;

use crate::config::{self, Config};
use crate::error::Result;
use crate::json::{now, num, read_json, read_text, remove, write_text};
use crate::paths::Paths;
use crate::process::{pid_alive, spawn_self};
use crate::state::{self, SPEAK_ERROR, SPEECH_DONE, TTS_PID, TTS_STOP};

/// Text for the speech service: no markdown marks, links as "link", one line, at most `limit`.
pub fn clean(text: &str, limit: usize) -> String {
    static MARKS: OnceLock<Option<Regex>> = OnceLock::new();
    static LINKS: OnceLock<Option<Regex>> = OnceLock::new();
    let marks = MARKS.get_or_init(|| Regex::new(r"[`*_#>|]").ok());
    let links = LINKS.get_or_init(|| Regex::new(r"https?://\S+").ok());
    let mut out = text.to_string();
    if let Some(re) = marks {
        out = re.replace_all(&out, "").into_owned();
    }
    if let Some(re) = links {
        out = re.replace_all(&out, "link").into_owned();
    }
    crate::text::truncate_chars(&crate::text::squash(&out), limit)
}

pub fn request_stop(paths: &Paths) {
    state::write(paths, TTS_STOP, &json!({ "at": now() }));
}

pub fn stopped(paths: &Paths, since: f64) -> bool {
    num(&read_json(&paths.state_file(TTS_STOP), serde_json::Value::Null), "at") > since
}

/// Starts `achaja speak` for the text in the background (hooks, reminders, mail reports).
pub fn say(paths: &Paths, text: &str) {
    if !text.trim().is_empty() {
        let _ = spawn_self(&["speak", text], &paths.data);
    }
}

/// Waits (up to two minutes) until no other speech process plays, then takes the turn.
fn wait_turn(paths: &Paths) {
    let me = std::process::id();
    let file = paths.state_file(TTS_PID);
    let deadline = now() + 120.0;
    while now() < deadline {
        let other = read_text(&file).and_then(|t| t.trim().parse::<u32>().ok());
        match other {
            Some(pid) if pid != me && pid_alive(pid) => std::thread::sleep(std::time::Duration::from_millis(300)),
            _ => break,
        }
    }
    let _ = write_text(&file, &me.to_string());
}

fn release_turn(paths: &Paths) {
    let file = paths.state_file(TTS_PID);
    if read_text(&file).map(|t| t.trim() == std::process::id().to_string()).unwrap_or(false) {
        remove(&file);
    }
}

/// Plays MP3 bytes on the current output; a Cast speaker that does not answer falls back to the
/// first local output, so nothing is lost (the error lands in state\speak_error.txt).
pub fn play(paths: &Paths, cfg: &Config, mp3: &[u8], since: f64) -> Result<()> {
    let out = super::outputs::current(paths, cfg);
    let stop = || stopped(paths, since);
    if out.kind == "cast" {
        match crate::cast::play(mp3, &out.cast_name, &out.host, &stop) {
            Ok(()) => return Ok(()),
            Err(e) => {
                let _ = write_text(&paths.state_file(SPEAK_ERROR), &format!("{}: {e}", chrono::Local::now().format("%c")));
                let (list, _) = super::outputs::outputs(cfg);
                let local = list.iter().find(|o| o.kind == "local").map(|o| o.device.clone()).unwrap_or_default();
                return super::playback::play_mp3(mp3, &local, &stop);
            }
        }
    }
    super::playback::play_mp3(mp3, &out.device, &stop)
}

/// The command itself.
pub fn run(paths: &Paths, text: &str) -> Result<()> {
    let since = now();
    let cfg = config::load_or_default(paths);
    let text = clean(text, cfg.f64_or("voice.max_chars", 4000.0) as usize);
    if text.is_empty() {
        return Ok(());
    }
    let voice = cfg.str_or("voice.tts_voice", "pl-PL-ZofiaNeural");
    let rate = cfg.str_or("voice.rate", "+0%");
    let result = crate::tts::synth_cached(paths, &text, &voice, &rate).and_then(|mp3| {
        wait_turn(paths);
        if !stopped(paths, since) {
            play(paths, &cfg, &mp3, since)?;
            super::outputs::consume_once(paths);
        }
        Ok(())
    });
    state::stamp(paths, SPEECH_DONE);
    release_turn(paths);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_markdown_and_links() {
        assert_eq!(clean("**Gotowe** zobacz https://x.pl/a  `kod`", 100), "Gotowe zobacz link kod");
        assert_eq!(clean("abcdef", 3), "abc");
    }
}
