//! Updates: the window's updater (Tauri, signed latest.json on GitHub Releases) does the work;
//! here are `achaja update check | install` for the voice session (install asks the running
//! window to install now) and "what's new" after an update (said once, listed in the chat).

use clap::Subcommand;
use serde_json::{json, Value};

use crate::error::Result;
use crate::json::{now, read_value, write_json};
use crate::paths::Paths;
use crate::state::{self, UPDATE_REQUEST};

pub const LATEST: &str = "https://github.com/Banzamel/AchajaVA/releases/latest/download/latest.json";
pub const RELEASES: &str = "https://github.com/Banzamel/AchajaVA/releases";

#[derive(Debug, Subcommand)]
pub enum UpdateCmd {
    /// Czy jest nowsza wersja (JSON)
    Check,
    /// Instaluje nowa wersje (okno Achai pobiera ja i uruchamia ponownie)
    Install,
}

/// "v0.14.2" -> [0, 14, 2]; suffixes (-dev, -rc1) are ignored.
pub fn version_key(text: &str) -> Vec<u64> {
    let t = text.trim().trim_start_matches('v');
    let digits: String = t.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    digits.split('.').filter(|p| !p.is_empty()).filter_map(|p| p.parse().ok()).collect()
}

pub fn newer(candidate: &str, current: &str) -> bool {
    version_key(candidate) > version_key(current)
}

async fn latest() -> Result<Value> {
    Ok(crate::runtime::http(15)?.get(LATEST).send().await?.error_for_status()?.json().await?)
}

/// Remembers the notes of a downloaded version, to say after the update what changed.
pub fn save_notes(paths: &Paths, version: &str, notes: &str) {
    let file = paths.state_file("update_notes.json");
    if read_value(&file)["version"].as_str() != Some(version) {
        let _ = write_json(&file, &json!({"version": version, "notes": notes, "announced": false}));
    }
}

/// After the first start of a new version: (sentence to say, note for the chat), once.
pub fn whats_new(paths: &Paths, lang: &str) -> Option<(String, String)> {
    let file = paths.state_file("update_notes.json");
    let mut data = read_value(&file);
    if data["version"].as_str() != Some(crate::VERSION) || data["announced"] == json!(true) {
        return None;
    }
    data["announced"] = json!(true);
    let _ = write_json(&file, &data);
    let notes = data["notes"].as_str().unwrap_or("");
    let title = notes.lines().find(|l| l.starts_with('#')).map(|l| l.trim_start_matches('#').trim().to_string()).unwrap_or_else(|| format!("Achaja {}", crate::VERSION));
    let points: Vec<String> = notes
        .lines()
        .filter_map(|l| l.trim_start().strip_prefix("- ").map(str::trim))
        .map(|l| l.split("**").nth(1).unwrap_or(l).replace(['*', '`'], ""))
        .take(8)
        .collect();
    let note = title + &points.iter().map(|p| format!("\n• {p}")).collect::<String>();
    let spoken = if lang == "en" {
        format!("I've updated to version {}. What's new is in the chat window.", crate::VERSION)
    } else {
        format!("Zaktualizowałam się do wersji {}. Co nowego, masz w oknie rozmowy.", crate::VERSION)
    };
    Some((spoken, note))
}

pub fn run(paths: &Paths, cmd: UpdateCmd) -> Result<i32> {
    let info = crate::runtime::block_on(latest());
    let available = info.as_ref().ok().and_then(|v| v["version"].as_str().map(str::to_string)).filter(|v| newer(v, crate::VERSION));
    match cmd {
        UpdateCmd::Check => {
            let notes = info.as_ref().ok().and_then(|v| v["notes"].as_str().map(|n| crate::text::truncate_chars(n, 2000)));
            let avail = available.as_ref().map(|v| json!({"version": v, "page": RELEASES, "notes": notes}));
            println!("{}", serde_json::to_string_pretty(&json!({"current": crate::VERSION, "available": avail, "self_update": true}))?);
        }
        UpdateCmd::Install => match available {
            None => println!("Achaja {} jest aktualna.", crate::VERSION),
            Some(v) => {
                state::write(paths, UPDATE_REQUEST, &json!({"at": now(), "version": v}));
                println!("Okno Achai pobierze i zainstaluje wersje {v}, zamknie sie i uruchomi ponownie w nowej wersji.");
            }
        },
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions() {
        assert!(newer("0.14.0", "0.13.2"));
        assert!(newer("v0.13.10", "0.13.9"));
        assert!(!newer("0.13.2", "0.13.2"));
        assert!(!newer("0.13.2-rc1", "0.13.2"));
    }
}
