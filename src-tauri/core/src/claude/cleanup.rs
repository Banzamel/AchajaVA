//! Tidying Claude Code's records: deleting a session, all sessions of a project, or everything
//! older than N days (sessions, debug logs, shell snapshots, todo lists, file history). Only
//! Claude Code's own files in ~/.claude, never project files; never an active session or
//! Achaja's current conversation.

use std::path::{Path, PathBuf};

use super::sessions::{projects, projects_dir, sessions, size_of, Session};
use super::trust::claude_home;
use crate::error::{AppError, Result};
use crate::json::{mtime, now, read_value, write_json};
use crate::paths::Paths;

const CLEAN_DIRS: &[&str] = &["debug", "shell-snapshots", "todos", "file-history"];

fn remove(path: &Path) {
    if path.is_dir() {
        let _ = std::fs::remove_dir_all(path);
    } else {
        let _ = std::fs::remove_file(path);
    }
}

/// [(name, size)] of the folders in ~/.claude, largest first (without the credentials).
pub fn storage() -> Vec<(String, u64)> {
    let mut rows: Vec<(String, u64)> = std::fs::read_dir(claude_home())
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_name() != ".credentials.json")
        .map(|e| (e.file_name().to_string_lossy().into_owned(), size_of(&e.path())))
        .collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1));
    rows
}

/// Everything Claude Code keeps about one session.
fn session_files(project_id: &str, sid: &str) -> Vec<PathBuf> {
    let home = claude_home();
    let mut paths = vec![
        projects_dir().join(project_id).join(format!("{sid}.jsonl")),
        projects_dir().join(project_id).join(sid),
        home.join("file-history").join(sid),
        home.join("session-env").join(sid),
    ];
    paths.extend(
        std::fs::read_dir(home.join("todos"))
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().starts_with(sid))
            .map(|e| e.path()),
    );
    paths.into_iter().filter(|p| p.exists()).collect()
}

/// An agent whose session is deleted starts a new conversation instead of resuming it.
fn forget_agent(paths: &Paths, sid: &str) {
    let file = paths.agents().join("sessions.json");
    let mut data = read_value(&file);
    if let Some(map) = data.as_object_mut() {
        let before = map.len();
        map.retain(|_, v| v.as_str() != Some(sid));
        if map.len() != before {
            let _ = write_json(&file, &data);
        }
    }
}

fn delete_known(paths: &Paths, project_id: &str, s: &Session, force: bool) -> Result<u64> {
    if s.current {
        return Err(AppError::msg("To biezaca rozmowa Achai - nie usuwam jej."));
    }
    if s.active && !force {
        return Err(AppError::msg("Ta sesja wlasnie pracuje (zapis w ostatnich 2 minutach) - nie usuwam jej."));
    }
    let mut freed = 0;
    for p in session_files(project_id, &s.id) {
        freed += size_of(&p);
        remove(&p);
    }
    forget_agent(paths, &s.id);
    Ok(freed)
}

pub fn delete_session(paths: &Paths, project_id: &str, sid: &str) -> Result<u64> {
    let list = sessions(paths, project_id);
    let s = list
        .iter()
        .find(|s| s.id == sid)
        .ok_or_else(|| AppError::msg(format!("Nie ma sesji {sid} w projekcie {project_id}.")))?;
    delete_known(paths, project_id, s, false)
}

pub fn delete_project(paths: &Paths, project_id: &str) -> Result<u64> {
    let list = sessions(paths, project_id);
    if list.iter().any(|s| s.current || s.active) {
        return Err(AppError::msg("W tym projekcie jest aktywna sesja albo biezaca rozmowa Achai - usun pojedyncze sesje."));
    }
    let mut freed = 0;
    for s in &list {
        freed += delete_known(paths, project_id, s, true)?;
    }
    let dir = projects_dir().join(project_id);
    freed += size_of(&dir);
    remove(&dir);
    Ok(freed)
}

/// (sessions, other files, bytes) older than `days`; deleted only with `apply`.
pub fn clean(paths: &Paths, days: u64, apply: bool) -> (usize, usize, u64) {
    let limit = now() - days as f64 * 86400.0;
    let (mut n_sessions, mut n_files, mut freed) = (0, 0, 0);
    for p in projects() {
        for s in sessions(paths, &p.id) {
            if s.mtime < limit && !s.current && !s.active {
                n_sessions += 1;
                freed += s.size;
                if apply {
                    let _ = delete_known(paths, &p.id, &s, false);
                }
            }
        }
        let dir = projects_dir().join(&p.id);
        if apply && std::fs::read_dir(&dir).map(|mut d| d.next().is_none()).unwrap_or(false) {
            let _ = std::fs::remove_dir(&dir);
        }
    }
    for name in CLEAN_DIRS {
        for entry in std::fs::read_dir(claude_home().join(name)).into_iter().flatten().flatten() {
            let path = entry.path();
            if mtime(&path) < limit {
                n_files += 1;
                freed += size_of(&path);
                if apply {
                    remove(&path);
                }
            }
        }
    }
    (n_sessions, n_files, freed)
}
