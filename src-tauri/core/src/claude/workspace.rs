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

/// Everything of the workspace except the folder trust: Achaja's CLAUDE.md, skills and
/// settings.json (the user's CLAUDE.local.md, settings.local.json and own skills stay as they are),
/// and achaja.cmd.
pub fn write_files(paths: &Paths, cfg: &Config) -> Result<()> {
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
    Ok(())
}

pub fn prepare(paths: &Paths, cfg: &Config) -> Result<()> {
    write_files(paths, cfg)?;
    super::trust::trust(&paths.data)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_the_templates_and_keeps_local_files() {
        let (data, res) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let r = res.path().join("claude");
        std::fs::create_dir_all(r.join("skills").join("poczta")).unwrap();
        std::fs::write(r.join("CLAUDE.md"), "rules").unwrap();
        std::fs::write(r.join("skills").join("poczta").join("SKILL.md"), "mail").unwrap();
        std::fs::write(r.join("settings.json"), r#"{"hooks": {"x": "__ACHAJA_EXE__ hook gate"}}"#).unwrap();
        let p = Paths::at(data.path(), res.path());
        let dir = p.claude_dir();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("CLAUDE.local.md"), "mine").unwrap();
        std::fs::write(dir.join("settings.local.json"), "{}").unwrap();
        write_files(&p, &Config::new(Value::Null)).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("CLAUDE.md")).unwrap(), "rules");
        assert!(dir.join("skills").join("poczta").join("SKILL.md").is_file());
        assert!(read_value(&dir.join("settings.json"))["hooks"]["x"].as_str().unwrap().ends_with(" hook gate"));
        assert_eq!(std::fs::read_to_string(dir.join("CLAUDE.local.md")).unwrap(), "mine");
        assert_eq!(std::fs::read_to_string(dir.join("settings.local.json")).unwrap(), "{}");
        assert!(data.path().join("achaja.cmd").is_file());
    }
}
