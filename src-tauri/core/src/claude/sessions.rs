//! Claude Code sessions on this PC (~/.claude/projects): projects with their size and last work,
//! the sessions of a project with a title, and which one is active or Achaja's current one.

use std::collections::HashMap;
use std::io::BufRead;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use super::trust::claude_home;
use crate::json::{mtime, now, read_value};
use crate::paths::Paths;

pub const ACTIVE_SECONDS: f64 = 120.0;

#[derive(Debug, Clone, Serialize)]
pub struct Project {
    pub id: String,
    pub path: String,
    pub size: u64,
    pub sessions: usize,
    pub last: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub messages: usize,
    pub size: u64,
    pub mtime: f64,
    pub active: bool,
    pub current: bool,
    pub agent: String,
}

pub fn projects_dir() -> PathBuf {
    claude_home().join("projects")
}

pub fn size_of(path: &Path) -> u64 {
    let Ok(meta) = std::fs::symlink_metadata(path) else { return 0 };
    if meta.is_file() {
        return meta.len();
    }
    if !meta.is_dir() {
        return 0;
    }
    std::fs::read_dir(path).map(|it| it.flatten().map(|e| size_of(&e.path())).sum()).unwrap_or(0)
}

pub fn human(n: u64) -> String {
    let mut v = n as f64;
    for unit in ["B", "KB", "MB", "GB"] {
        if v < 1024.0 || unit == "GB" {
            return if unit == "B" { format!("{v:.0} {unit}") } else { format!("{v:.1} {unit}") };
        }
        v /= 1024.0;
    }
    format!("{v:.1} GB")
}

pub struct Info {
    pub title: String,
    pub cwd: String,
    pub messages: usize,
}

/// Title (summary or the first user message), working folder and message count of a session.
pub fn session_info(path: &Path) -> Info {
    let (mut title, mut summary, mut cwd, mut messages) = (String::new(), String::new(), String::new(), 0);
    if let Ok(file) = std::fs::File::open(path) {
        for line in std::io::BufReader::new(file).lines().take(4000).map_while(Result::ok) {
            let Ok(rec) = serde_json::from_str::<Value>(&line) else { continue };
            if cwd.is_empty() {
                cwd = rec["cwd"].as_str().unwrap_or("").to_string();
            }
            let kind = rec["type"].as_str().unwrap_or("");
            if kind == "summary" && summary.is_empty() {
                summary = rec["summary"].as_str().unwrap_or("").to_string();
            }
            let meta = rec["isMeta"].as_bool().unwrap_or(false);
            if (kind == "user" || kind == "assistant") && !meta {
                messages += 1;
            }
            if title.is_empty() && kind == "user" && !meta {
                let content = &rec["message"]["content"];
                let text = match content {
                    Value::Array(parts) => parts.iter().filter_map(|p| p["text"].as_str()).collect::<Vec<_>>().join(" "),
                    other => other.as_str().unwrap_or("").to_string(),
                };
                let text = crate::text::squash(&text);
                if !text.is_empty() && !text.starts_with(['<']) && !text.starts_with("[Request interrupted") && !text.starts_with("Caveat:") {
                    title = crate::text::truncate_chars(&text, 120);
                }
            }
        }
    }
    let title = if !summary.is_empty() { summary } else if !title.is_empty() { title } else { "(bez tytułu)".into() };
    Info { title, cwd, messages }
}

fn jsonl(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .map(|it| it.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "jsonl")).collect())
        .unwrap_or_default()
}

fn newest(files: &[PathBuf]) -> Option<&PathBuf> {
    files.iter().max_by(|a, b| mtime(a).total_cmp(&mtime(b)))
}

fn same_folder(a: &str, b: &Path) -> bool {
    let norm = |s: &str| s.replace('/', "\\").trim_end_matches('\\').to_lowercase();
    !a.is_empty() && norm(a) == norm(&b.to_string_lossy())
}

/// Achaja's current conversation: the newest session whose working folder is the data folder.
pub fn achaja_current(paths: &Paths) -> Option<String> {
    let mut best: Option<PathBuf> = None;
    for entry in std::fs::read_dir(projects_dir()).into_iter().flatten().flatten() {
        let files = jsonl(&entry.path());
        let Some(top) = newest(&files) else { continue };
        if same_folder(&session_info(top).cwd, &paths.data) && best.as_ref().map_or(true, |b| mtime(top) > mtime(b)) {
            best = Some(top.clone());
        }
    }
    best.and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
}

/// Session id -> project name of Achaja's project agents.
pub fn agent_sessions(paths: &Paths) -> HashMap<String, String> {
    let v = read_value(&paths.agents().join("sessions.json"));
    v.as_object()
        .map(|o| o.iter().filter_map(|(name, sid)| sid.as_str().map(|s| (s.to_string(), name.clone()))).collect())
        .unwrap_or_default()
}

pub fn projects() -> Vec<Project> {
    let mut out: Vec<Project> = std::fs::read_dir(projects_dir())
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| {
            let dir = e.path();
            let files = jsonl(&dir);
            let top = newest(&files);
            let cwd = top.map(|t| session_info(t).cwd).unwrap_or_default();
            let id = e.file_name().to_string_lossy().into_owned();
            Project {
                path: if cwd.is_empty() { id.clone() } else { cwd },
                size: size_of(&dir),
                sessions: files.len(),
                last: top.map(|t| mtime(t)).unwrap_or_else(|| mtime(&dir)),
                id,
            }
        })
        .collect();
    out.sort_by(|a, b| b.last.total_cmp(&a.last));
    out
}

pub fn sessions(paths: &Paths, project_id: &str) -> Vec<Session> {
    let dir = projects_dir().join(project_id);
    let (current, agents, t) = (achaja_current(paths), agent_sessions(paths), now());
    let mut out: Vec<Session> = jsonl(&dir)
        .iter()
        .map(|f| {
            let id = f.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let info = session_info(f);
            let extra = dir.join(&id); // subagents and tool results
            let m = mtime(f);
            Session {
                title: info.title,
                messages: info.messages,
                size: size_of(f) + size_of(&extra),
                mtime: m,
                active: t - m < ACTIVE_SECONDS,
                current: current.as_deref() == Some(id.as_str()),
                agent: agents.get(&id).cloned().unwrap_or_default(),
                id,
            }
        })
        .collect();
    out.sort_by(|a, b| b.mtime.total_cmp(&a.mtime));
    out
}
