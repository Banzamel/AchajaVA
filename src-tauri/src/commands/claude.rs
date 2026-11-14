//! The Claude tab: the account, plan usage, token statistics, sessions on this PC and tidying them. Deleting is
//! the user's own click confirmed in the window, so it does not go through the session's gate.

use std::sync::Arc;

use achaja_core::claude::account::{self, Account, UsageWindow};
use achaja_core::claude::cleanup;
use achaja_core::claude::sessions::{self, Project, Session};
use achaja_core::error::{AppError, Result};
use achaja_core::paths::Paths;
use serde::Serialize;
use tauri::State;

use crate::host::AppState;

type St<'a> = State<'a, Arc<AppState>>;

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| AppError::msg(e.to_string()))
}

#[tauri::command]
pub async fn claude_account() -> Result<Account> {
    blocking(account::account).await
}

/// The usage windows; with `cached` the last reading on disk (instant).
#[tauri::command]
pub async fn claude_usage(st: St<'_>, cached: bool) -> Result<Vec<UsageWindow>> {
    if cached {
        return Ok(account::cached_usage(&st.paths));
    }
    account::usage(&st.paths, false).await
}

/// Token use and the estimated cost from state	okens.jsonl (the statistics section).
#[tauri::command]
pub async fn claude_stats(st: St<'_>) -> Result<achaja_core::tokens::stats::Stats> {
    let paths: Paths = st.paths.clone();
    blocking(move || achaja_core::tokens::stats::load(&paths)).await
}

#[derive(Serialize)]
pub struct Projects {
    projects: Vec<Project>,
    total: u64,
}

#[tauri::command]
pub async fn claude_projects() -> Result<Projects> {
    blocking(|| Projects { projects: sessions::projects(), total: sessions::size_of(&achaja_core::claude::trust::claude_home()) }).await
}

#[tauri::command]
pub async fn claude_sessions(st: St<'_>, project: String) -> Result<Vec<Session>> {
    let paths: Paths = st.paths.clone();
    blocking(move || sessions::sessions(&paths, &project)).await
}

#[derive(Serialize)]
pub struct Deleted {
    freed: u64,
    errors: Vec<String>,
}

/// projects: whole projects; sessions: [project, session] pairs.
#[tauri::command]
pub async fn claude_delete(st: St<'_>, projects: Vec<String>, sessions: Vec<(String, String)>) -> Result<Deleted> {
    let paths = st.paths.clone();
    blocking(move || {
        let mut out = Deleted { freed: 0, errors: Vec::new() };
        for p in &projects {
            match cleanup::delete_project(&paths, p) {
                Ok(n) => out.freed += n,
                Err(e) => out.errors.push(e.to_string()),
            }
        }
        for (p, s) in sessions.iter().filter(|(p, _)| !projects.contains(p)) {
            match cleanup::delete_session(&paths, p, s) {
                Ok(n) => out.freed += n,
                Err(e) => out.errors.push(e.to_string()),
            }
        }
        out
    })
    .await
}

#[tauri::command]
pub async fn claude_clean_preview(st: St<'_>, days: u64) -> Result<(usize, usize, u64)> {
    let paths = st.paths.clone();
    blocking(move || cleanup::clean(&paths, days, false)).await
}

#[tauri::command]
pub async fn claude_clean(st: St<'_>, days: u64) -> Result<u64> {
    let paths = st.paths.clone();
    blocking(move || cleanup::clean(&paths, days, true).2).await
}

#[tauri::command]
pub fn claude_open_dir(project: Option<String>) -> Result<()> {
    let base = achaja_core::claude::trust::claude_home();
    let dir = match project {
        Some(p) if !p.contains(['/', '\\']) => sessions::projects_dir().join(p),
        _ => base,
    };
    achaja_core::process::hidden("explorer.exe").arg(dir).spawn()?;
    Ok(())
}
