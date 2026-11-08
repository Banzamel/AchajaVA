//! The Python versions kept their data in the install folder itself (config.json, state\,
//! models\, .mcp.json, .claude\settings.local.json, .claude\CLAUDE.local.md). The installer moves
//! them into data\ while it replaces the old version; this is the safety net when they are still
//! next to the program: copied into data\ (never over newer data), checked, then removed.

use std::path::Path;

use achaja_core::paths::Paths;

const FILES: &[&str] = &["config.json", ".mcp.json", ".claude/settings.local.json", ".claude/CLAUDE.local.md"];
const DIRS: &[&str] = &["state", "models"];

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    if from.is_dir() {
        std::fs::create_dir_all(to)?;
        for e in std::fs::read_dir(from)?.flatten() {
            copy_tree(&e.path(), &to.join(e.file_name()))?;
        }
    } else if !to.exists() {
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(from, to)?;
    }
    Ok(())
}

pub fn from_python(p: &Paths) {
    let old = &p.install;
    // only in an installed copy whose data folder has no settings yet
    if old == &p.data || !old.join("config.json").is_file() || p.config().exists() || old.join("unins000.exe").exists() {
        return;
    }
    let mut ok = true;
    for rel in FILES.iter().chain(DIRS) {
        let src = old.join(rel);
        if src.exists() && copy_tree(&src, &p.data.join(rel)).is_err() {
            ok = false;
        }
    }
    if !ok || !p.config().is_file() {
        return; // the originals stay; nothing is lost
    }
    for rel in FILES {
        let _ = std::fs::remove_file(old.join(rel));
    }
    for rel in DIRS {
        let _ = std::fs::remove_dir_all(old.join(rel));
    }
    let _ = std::fs::remove_dir(old.join(".claude"));
}
