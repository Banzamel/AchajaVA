//! The Agents tab: working and finished project agents, the log of one, stop and remove.

use std::sync::Arc;

use achaja_core::agents;
use achaja_core::error::Result;
use achaja_core::json::{mtime, now, read_text};
use serde::Serialize;
use tauri::State;

use crate::host::AppState;

#[derive(Serialize)]
pub struct AgentRow {
    name: String,
    working: bool,
    minutes: u64,
    finished_at: f64,
}

#[tauri::command]
pub fn agents_list(st: State<'_, Arc<AppState>>) -> Vec<AgentRow> {
    let dir = st.paths.agents();
    let running = agents::running(&st.paths);
    let mut rows: Vec<AgentRow> = running
        .iter()
        .map(|(n, r)| AgentRow { name: n.clone(), working: true, minutes: ((now() - r["started"].as_f64().unwrap_or(now())) / 60.0).round() as u64, finished_at: 0.0 })
        .collect();
    let mut logs: Vec<_> = std::fs::read_dir(&dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "log")).collect();
    logs.sort_by(|a, b| mtime(b).total_cmp(&mtime(a)));
    for log in logs {
        let name = log.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        if !running.contains_key(&name) {
            rows.push(AgentRow { finished_at: mtime(&log), name, working: false, minutes: 0 });
        }
    }
    rows
}

/// (log text, its modification time) of one agent.
#[tauri::command]
pub fn agent_log(st: State<'_, Arc<AppState>>, name: String) -> (String, f64) {
    let file = st.paths.agents().join(format!("{}.log", name.replace(['/', '\\'], "")));
    (read_text(&file).unwrap_or_default(), mtime(&file))
}

#[tauri::command]
pub async fn agent_stop(st: State<'_, Arc<AppState>>, name: String) -> Result<bool> {
    let paths = st.paths.clone();
    Ok(tauri::async_runtime::spawn_blocking(move || agents::stop(&paths, &name)).await.unwrap_or(false))
}

/// A finished agent leaves the list; its log goes to state\agents\archive.
#[tauri::command]
pub fn agent_remove(st: State<'_, Arc<AppState>>, name: String) -> Result<()> {
    let dir = st.paths.agents();
    let name = name.replace(['/', '\\'], "");
    std::fs::create_dir_all(dir.join("archive"))?;
    std::fs::rename(dir.join(format!("{name}.log")), dir.join("archive").join(format!("{name}.log")))?;
    Ok(())
}
