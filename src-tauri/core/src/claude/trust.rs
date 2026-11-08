//! Claude Code's own files in the user profile (~/.claude, ~/.claude.json): where they are, and
//! marking Achaja's folder as trusted (projects[<folder>].hasTrustDialogAccepted). Without it
//! Claude Code skips .claude\settings.local.json (the enabled MCP servers) and the hidden
//! session would ask about the server from .mcp.json in a window nobody sees.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::error::Result;
use crate::json::{read_text, write_text};

pub fn user_home() -> PathBuf {
    std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from).unwrap_or_default()
}

/// ~/.claude (or CLAUDE_CONFIG_DIR).
pub fn claude_home() -> PathBuf {
    std::env::var_os("CLAUDE_CONFIG_DIR").map(PathBuf::from).unwrap_or_else(|| user_home().join(".claude"))
}

/// ~/.claude.json (inside CLAUDE_CONFIG_DIR when that is set).
pub fn global_config() -> PathBuf {
    match std::env::var_os("CLAUDE_CONFIG_DIR") {
        Some(dir) => PathBuf::from(dir).join(".claude.json"),
        None => user_home().join(".claude.json"),
    }
}

pub fn trust(folder: &Path) -> Result<()> {
    let key = folder.to_string_lossy().replace('\\', "/");
    let path = global_config();
    let mut data: Value = match read_text(&path) {
        Some(text) => match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => return Ok(()), // a file we cannot read is left alone
        },
        None => json!({}),
    };
    if data["projects"][&key]["hasTrustDialogAccepted"] == json!(true) {
        return Ok(());
    }
    if !data["projects"].is_object() {
        data["projects"] = json!({});
    }
    if !data["projects"][&key].is_object() {
        data["projects"][&key] = json!({});
    }
    data["projects"][&key]["hasTrustDialogAccepted"] = json!(true);
    if path.exists() {
        let _ = std::fs::copy(&path, path.with_extension("json.achaja-backup"));
    }
    write_text(&path, &serde_json::to_string_pretty(&data)?)
}
