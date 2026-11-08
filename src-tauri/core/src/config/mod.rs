//! config.json: one file for all languages. The control words, speech model, voice and phrases
//! come from languages[<language>]; values written straight into "wake"/"voice" win.

mod access;

pub use access::Config;

use serde_json::{Map, Value};

use crate::error::Result;
use crate::json::{read_text, write_text};
use crate::paths::Paths;

const WAKE_KEYS: &[&str] = &["model", "phrases", "end", "cancel", "clear"];
const VOICE_KEYS: &[&str] = &["tts_voice", "ack_phrases", "working_phrases", "wake_replies"];
pub const DEFAULT_MODEL: &str = "vosk-model-small-pl-0.22";

/// The template shipped with the program (also the source of a new config.json).
/// The installer writes the language chosen for it into data\install-language.
pub fn example(paths: &Paths) -> String {
    let text = read_text(&paths.bundled("claude/config.example.json")).unwrap_or_else(|| "{}".into());
    match read_text(&paths.data.join("install-language")).as_deref().map(str::trim) {
        Some("en") => text.replacen("\"language\": \"pl\"", "\"language\": \"en\"", 1),
        _ => text,
    }
}

/// config.json as written on disk; created from the template when missing.
pub fn load_raw(paths: &Paths) -> Result<Value> {
    let file = paths.config();
    if !file.exists() {
        write_text(&file, &example(paths))?;
    }
    let text = read_text(&file).unwrap_or_default();
    Ok(serde_json::from_str(&text)?)
}

/// config.json, or the template when there is none yet (without creating the file).
pub fn peek_raw(paths: &Paths) -> Value {
    let text = read_text(&paths.config()).unwrap_or_else(|| example(paths));
    serde_json::from_str(&text).unwrap_or(Value::Null)
}

/// Like load_or_default(), without creating config.json.
pub fn peek(paths: &Paths) -> Config {
    Config::new(merge_language(peek_raw(paths)))
}

pub fn save_raw(paths: &Paths, cfg: &Value) -> Result<()> {
    write_text(&paths.config(), &(serde_json::to_string_pretty(cfg)? + "\n"))
}

/// The configuration with the language block merged in.
pub fn load(paths: &Paths) -> Result<Config> {
    Ok(Config::new(merge_language(load_raw(paths)?)))
}

/// Like load(), but an unreadable file gives the template (hooks must never fail on it).
pub fn load_or_default(paths: &Paths) -> Config {
    load(paths).unwrap_or_else(|_| {
        Config::new(merge_language(serde_json::from_str(&example(paths)).unwrap_or(Value::Null)))
    })
}

pub fn merge_language(mut cfg: Value) -> Value {
    if !cfg.is_object() {
        cfg = Value::Object(Map::new());
    }
    let lang = cfg.get("language").and_then(Value::as_str).unwrap_or("pl").to_string();
    let block = cfg.get("languages").and_then(|l| l.get(&lang)).cloned().unwrap_or(Value::Null);
    for (section, keys) in [("wake", WAKE_KEYS), ("voice", VOICE_KEYS)] {
        let target = cfg
            .as_object_mut()
            .map(|o| o.entry(section).or_insert_with(|| Value::Object(Map::new())));
        let Some(Value::Object(target)) = target else { continue };
        for key in keys {
            if let Some(value) = block.get(*key) {
                target.entry(key.to_string()).or_insert_with(|| value.clone());
            }
        }
    }
    cfg
}

/// The Vosk model folder: a name inside models\ or a full path (wake.model).
pub fn model_dir(paths: &Paths, cfg: &Config) -> std::path::PathBuf {
    let name = cfg.str_or("wake.model", DEFAULT_MODEL);
    let path = std::path::PathBuf::from(&name);
    if path.is_absolute() {
        path
    } else {
        paths.models().join(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_block_fills_missing_words() {
        let cfg = merge_language(serde_json::json!({
            "language": "en", "wake": {"end": ["go"]},
            "languages": {"en": {"end": ["execute"], "cancel": ["cancel"], "tts_voice": "en-US-JennyNeural"}}
        }));
        let c = Config::new(cfg);
        assert_eq!(c.strs("wake.end"), vec!["go"]);
        assert_eq!(c.strs("wake.cancel"), vec!["cancel"]);
        assert_eq!(c.str_or("voice.tts_voice", ""), "en-US-JennyNeural");
    }
}
