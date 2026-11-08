//! The hidden Claude session the listener drives: the one from the last start when it still runs
//! (and comes from this version), otherwise a new one that continues the conversation; and the
//! background helpers (mail watcher, phone alerts) that start with it.

use serde_json::json;

use super::Bridge;
use crate::claude::{launch, workspace};
use crate::config::Config;
use crate::error::Result;
use crate::paths::Paths;
use crate::process::{kill_tree, process_name, spawn_self};
use crate::state::{self, LISTENER};

pub fn attach_or_launch(paths: &Paths, cfg: &Config, bridge: &Bridge, mut resume: bool) -> Result<u32> {
    let last = state::read(paths, LISTENER);
    let mut pid = 0;
    if let Some(prev) = last["claude_pid"].as_u64().map(|p| p as u32).filter(|p| process_name(*p).as_deref() == Some("claude.exe")) {
        if last["version"].as_str() == Some(crate::VERSION) {
            pid = prev;
            bridge.log(&format!("Okno Achai juz jest otwarte (PID {pid}) - podpinam sie do niego."));
        } else {
            // a session from before the update has the old hooks and rules: restart, same conversation
            bridge.log(&format!("Sesja Claude (PID {prev}) pochodzi z poprzedniej wersji - uruchamiam ja od nowa."));
            kill_tree(prev);
            resume = true;
        }
    }
    if pid == 0 {
        if let Err(e) = workspace::prepare(paths, cfg) {
            bridge.log(&format!("BLAD przygotowania folderu Achai: {e}"));
        }
        pid = launch::launch(paths, cfg, resume, bridge.hidden)?;
        bridge.log(&format!("Uruchomiono Achaje {} (PID {pid}).", if bridge.hidden { "w tle" } else { "w nowym oknie" }));
    }
    state::write(paths, LISTENER, &json!({"listener_pid": std::process::id(), "claude_pid": pid, "version": crate::VERSION}));
    Ok(pid)
}

pub fn start_background(paths: &Paths, cfg: &Config, bridge: &Bridge) {
    if cfg.bool_or("mail.enabled", false) && !crate::mail::accounts::accounts(cfg, false).is_empty() {
        let _ = spawn_self(&["mail", "watch"], &paths.data);
        bridge.log("Nasluch poczty dziala w tle (achaja mail status).");
    }
    if cfg.bool_or("phone_alerts.enabled", false) {
        let _ = spawn_self(&["phone", "run"], &paths.data);
        bridge.log("Powiadomienia poczty na telefon dzialaja w tle (achaja phone status).");
    }
}
