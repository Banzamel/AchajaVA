//! The menu's system actions: editing config.json, backups, links, updates, and starting with
//! Windows (the HKCU Run entry; nothing is written into the user profile).

use std::sync::Arc;

use achaja_core::error::{AppError, Result};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::host::AppState;

type St<'a> = State<'a, Arc<AppState>>;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

#[tauri::command]
pub fn edit_config(st: St) -> Result<()> {
    achaja_core::config::load_raw(&st.paths)?; // creates it from the template when missing
    std::process::Command::new("notepad.exe").arg(st.paths.config()).spawn()?;
    Ok(())
}

/// (file name, file count) of a new backup.
#[tauri::command]
pub async fn backup_now(st: St<'_>) -> Result<(String, usize)> {
    let paths = st.paths.clone();
    let (file, n) = tauri::async_runtime::spawn_blocking(move || achaja_core::backup::create(&paths, "")).await.map_err(|e| AppError::msg(e.to_string()))??;
    Ok((file.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default(), n))
}

#[tauri::command]
pub fn open_backups(st: St) -> Result<()> {
    let folder = achaja_core::backup::settings(&st.paths).folder;
    std::fs::create_dir_all(&folder)?;
    std::process::Command::new("explorer.exe").arg(folder).spawn()?;
    Ok(())
}

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> Result<()> {
    if !url.starts_with("https://") {
        return Err(AppError::msg("only https links"));
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| AppError::msg(e.to_string()))
}

#[tauri::command]
pub fn update_check(app: AppHandle, st: St) {
    crate::updater::check(app, st.inner().clone(), true, false);
}

#[tauri::command]
pub fn update_install(app: AppHandle, st: St) {
    crate::commands::assistant::quit_app(&app, &st, true);
}

#[cfg(windows)]
pub fn autostart_get() -> bool {
    let key = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    key.open_subkey(RUN_KEY).and_then(|k| k.get_value::<String, _>("Achaja")).is_ok()
}

#[cfg(windows)]
pub fn autostart_set(on: bool) -> Result<()> {
    let key = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let (run, _) = key.create_subkey(RUN_KEY)?;
    if on {
        let exe = achaja_core::paths::paths().exe.clone();
        run.set_value("Achaja", &format!("\"{}\" --minimized", exe.display()))?;
    } else {
        let _ = run.delete_value("Achaja");
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn autostart_get() -> bool {
    false
}

#[cfg(not(windows))]
pub fn autostart_set(_on: bool) -> Result<()> {
    Ok(())
}
