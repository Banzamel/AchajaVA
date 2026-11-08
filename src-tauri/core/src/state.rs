//! The small state files in data\state through which Achaja's processes talk to each other:
//! the window and its listener, the Claude session hooks, speech, mail and background jobs.
//! The same names and shapes as in the Python versions, so nothing is lost on update.

use serde_json::{json, Value};

use crate::json::{now, num, read_json, read_text, remove, write_json, write_text};
use crate::paths::Paths;
use crate::process::pid_alive;

pub const WORK: &str = "achaja_state.json"; // {state: idle|busy|waiting, since, kind}
pub const APPROVAL: &str = "approval.json"; // the gate's yes for one turn
pub const FOLLOWUP: &str = "followup.json"; // Achaja asked a question: listen after speaking
pub const SPEECH_DONE: &str = "speech_done.json";
pub const TTS_PID: &str = "tts.pid";
pub const TTS_STOP: &str = "tts_stop.json";
pub const RECORDING: &str = "recording.json";
pub const VOICE_OUTPUT: &str = "voice_output.json";
pub const LAST_VOICE: &str = "last_voice.txt";
pub const LISTENER: &str = "listener.json";
pub const APP: &str = "app.json";
pub const SHOW_FLAG: &str = "app_show.json"; // a second start asks the running window to show
pub const SETTINGS_FLAG: &str = "app_settings.json"; // ... or to open its settings
pub const UPDATE_REQUEST: &str = "update_request.json"; // "achaja update install" from the session
pub const SPEAK_ERROR: &str = "speak_error.txt";

pub fn read(paths: &Paths, name: &str) -> Value {
    read_json(&paths.state_file(name), Value::Null)
}

pub fn write(paths: &Paths, name: &str, value: &Value) {
    let _ = write_json(&paths.state_file(name), value);
}

pub fn stamp(paths: &Paths, name: &str) {
    write(paths, name, &json!({ "at": now() }));
}

pub fn clear(paths: &Paths, name: &str) {
    remove(&paths.state_file(name));
}

pub fn at(paths: &Paths, name: &str) -> f64 {
    num(&read(paths, name), "at")
}

/// What the Achaja session does: "idle", "busy" or "waiting", and since when.
pub fn set_work(paths: &Paths, state: &str, kind: Option<&str>) {
    let mut value = json!({ "state": state, "since": now() });
    if let Some(kind) = kind {
        value["kind"] = json!(kind);
    }
    write(paths, WORK, &value);
}

pub fn work(paths: &Paths) -> (String, f64, String) {
    let v = read(paths, WORK);
    let state = v.get("state").and_then(Value::as_str).unwrap_or("idle").to_string();
    let kind = v.get("kind").and_then(Value::as_str).unwrap_or("").to_string();
    (state, num(&v, "since"), kind)
}

/// A speech process is playing (its PID is in tts.pid).
pub fn speaking(paths: &Paths) -> bool {
    read_text(&paths.state_file(TTS_PID))
        .and_then(|t| t.trim().parse::<u32>().ok())
        .map(pid_alive)
        .unwrap_or(false)
}

pub fn set_last_voice(paths: &Paths, text: &str) {
    let _ = write_text(&paths.state_file(LAST_VOICE), text);
}

/// The user is talking to Achaja (speech, recording, an answer awaited or work in progress):
/// background reports wait.
pub fn user_busy(paths: &Paths) -> bool {
    let t = now();
    if speaking(paths) || t - at(paths, RECORDING) < 150.0 || t - at(paths, FOLLOWUP) < 300.0 {
        return true;
    }
    let (state, since, _) = work(paths);
    (state == "busy" || state == "waiting") && t - since < 1200.0
}
