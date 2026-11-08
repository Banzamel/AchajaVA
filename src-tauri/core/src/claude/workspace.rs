//! The data folder as the Claude workspace of the Achaja session: Achaja's rules and skills
//! (refreshed from the program on every start), .claude\settings.json with the hooks pointing
//! at this Achaja.exe, achaja.cmd (the tools as one short command), and the folder marked as
//! trusted in Claude Code (otherwise the hidden session would ask about .mcp.json at start).

use std::path::Path;

use serde_json::Value;

use crate::config::Config;
use crate::error::Result;
use crate::json::{read_text, read_value, write_json, write_text};
use crate::paths::Paths;

fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    if from.is_dir() {
        std::fs::create_dir_all(to)?;
        for entry in std::fs::read_dir(from)?.flatten() {
            copy_tree(&entry.path(), &to.join(entry.file_name()))?;
        }
    } else if from.is_file() {
        std::fs::copy(from, to)?;
    }
    Ok(())
}

/// Replaces "__ACHAJA_EXE__" in every string of the template.
fn fill(value: &mut Value, exe: &str) {
    match value {
        Value::String(s) => *s = s.replace("__ACHAJA_EXE__", exe),
        Value::Array(items) => items.iter_mut().for_each(|v| fill(v, exe)),
        Value::Object(map) => map.values_mut().for_each(|v| fill(v, exe)),
        _ => {}
    }
}

/// The hooks settings for this install; the language chosen earlier is kept.
pub fn settings(paths: &Paths, cfg: &Config) -> Value {
    let mut template = read_value(&paths.bundled("claude/settings.json"));
    let exe = paths.exe.to_string_lossy().replace('\\', "/");
    fill(&mut template, &exe);
    let current = read_value(&paths.claude_dir().join("settings.json"));
    let language = current
        .get("language")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| if cfg.language() == "en" { "english".into() } else { "polish".into() });
    if let Value::Object(map) = &mut template {
        map.insert("language".into(), Value::String(language));
    }
    template
}

pub fn prepare(paths: &Paths, cfg: &Config) -> Result<()> {
    let dir = paths.claude_dir();
    std::fs::create_dir_all(&dir)?;
    copy_tree(&paths.bundled("claude/CLAUDE.md"), &dir.join("CLAUDE.md"))?;
    copy_tree(&paths.bundled("claude/skills"), &dir.join("skills"))?;
    let wanted = settings(paths, cfg);
    if read_value(&dir.join("settings.json")) != wanted {
        write_json(&dir.join("settings.json"), &wanted)?;
    }
    let cmd = format!("@echo off\r\n\"{}\" %*\r\n", paths.exe.display());
    if read_text(&paths.data.join("achaja.cmd")).as_deref() != Some(cmd.as_str()) {
        write_text(&paths.data.join("achaja.cmd"), &cmd)?;
    }
    super::trust::trust(&paths.data)?;
    Ok(())
}
