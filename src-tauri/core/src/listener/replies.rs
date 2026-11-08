//! The reply to the wake word ("Słucham.") — prepared in the background as trimmed clips kept
//! in memory, played to the end before recording starts, so it never lands in the prompt — and
//! giving claude.exe the configured microphone (Windows needs it to be recording at that moment,
//! and remembers the choice for the program).

use std::sync::{Arc, Mutex};

use rand::seq::IndexedRandom;
use serde_json::json;

use super::Bridge;
use crate::config::Config;
use crate::paths::Paths;
use crate::voice::clip::{trim_silence, Clip};

pub type Clips = Arc<Mutex<Vec<Clip>>>;

pub fn prepare(paths: &Paths, cfg: &Config, log: super::Sink, phrases: Vec<String>) -> Clips {
    let clips: Clips = Arc::new(Mutex::new(Vec::new()));
    let (out, paths, voice, rate) = (clips.clone(), paths.clone(), cfg.str_or("voice.tts_voice", "pl-PL-ZofiaNeural"), cfg.str_or("voice.rate", "+0%"));
    std::thread::spawn(move || {
        for text in phrases {
            match crate::tts::synth_cached(&paths, &text, &voice, &rate).and_then(|mp3| crate::voice::playback::decode(&mp3)) {
                Ok(clip) => {
                    if let Ok(mut c) = out.lock() {
                        c.push(trim_silence(&clip, 0.01));
                    }
                }
                Err(e) => log(format!("{} Nie udalo sie przygotowac odpowiedzi '{text}': {e}", chrono::Local::now().format("%H:%M:%S"))),
            }
        }
    });
    clips
}

/// Says the reply and waits for its end: the clip on a local output, otherwise `achaja speak`.
pub fn reply(paths: &Paths, cfg: &Config, bridge: &Bridge, phrases: &[String], clips: &Clips) {
    let out = crate::voice::outputs::current(paths, cfg);
    let clip = clips.lock().ok().and_then(|c| c.choose(&mut rand::rng()).cloned());
    if let (Some(clip), "local") = (clip, out.kind.as_str()) {
        match crate::voice::playback::play_clip(&clip, &out.device) {
            Ok(()) => return,
            Err(e) => bridge.log(&format!("Nie udalo sie odtworzyc odpowiedzi: {e}")),
        }
    }
    let Some(text) = phrases.choose(&mut rand::rng()) else { return };
    std::thread::sleep(std::time::Duration::from_millis(150)); // speech ordered before the last stop is skipped
    let child = crate::process::hidden(&paths.exe).args(["speak", text]).current_dir(&paths.data).spawn();
    if let Ok(mut c) = child {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        while std::time::Instant::now() < deadline {
            if c.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        bridge.log("Odpowiedz na wybudzenie trwala za dlugo - nagrywam bez niej.");
    }
}

pub fn assign_claude_microphone(paths: &Paths, bridge: &Bridge, pid: u32, mic: &str) {
    let flag = paths.state_file("claude_mic.json");
    if mic.is_empty() || crate::json::read_value(&flag)["mic"].as_str() == Some(mic) {
        return;
    }
    let (mic, flag, log) = (mic.to_string(), flag.clone(), bridge.sender());
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(1500));
        let stamp = chrono::Local::now().format("%H:%M:%S");
        match crate::audio_routing::assign(pid, "input", &mic) {
            Ok(_) => {
                let _ = crate::json::write_json(&flag, &json!({"mic": mic}));
                log(format!("{stamp} Przypisano mikrofon '{mic}' do claude.exe (na stale)."));
            }
            Err(e) => log(format!("{stamp} Nie udalo sie przypisac mikrofonu do claude.exe: {e}")),
        }
    });
}
