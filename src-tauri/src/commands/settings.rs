//! The Settings view: the form, saving it, and its helpers — audio devices, Cast speakers,
//! testing a mail account or Home Assistant, phones with the HA app, the speech model.

use std::sync::Arc;

use achaja_core::error::{AppError, Result};
use achaja_core::settings::form::AccountForm;
use achaja_core::settings::{self, SettingsForm};
use achaja_core::voice::devices::DeviceNames;
use serde_json::json;
use tauri::{AppHandle, Emitter, State};

use crate::host::AppState;

type St<'a> = State<'a, Arc<AppState>>;

#[tauri::command]
pub fn settings_load(st: St) -> Result<SettingsForm> {
    let editing = st.paths.config().exists() && !st.first_run.load(std::sync::atomic::Ordering::Relaxed);
    let raw = achaja_core::config::peek_raw(&st.paths);
    let mut form = settings::load(&st.paths, &raw, editing);
    form.autostart = crate::commands::system::autostart_get();
    Ok(form)
}

#[tauri::command]
pub fn settings_save(st: St, form: SettingsForm) -> Result<()> {
    // the listener cannot start without the speech model of the language: the window offers it
    let raw = achaja_core::config::peek_raw(&st.paths);
    if let Some(model) = settings::model_missing(&st.paths, &raw, &form.language) {
        return Err(AppError::msg(format!("model:{model}")));
    }
    settings::apply(&st.paths, &form)?;
    st.first_run.store(false, std::sync::atomic::Ordering::Relaxed);
    crate::commands::system::autostart_set(form.autostart)?;
    Ok(())
}

#[tauri::command]
pub async fn audio_devices() -> DeviceNames {
    tauri::async_runtime::spawn_blocking(achaja_core::voice::devices::list).await.unwrap_or_default()
}

#[tauri::command]
pub async fn cast_discover() -> Vec<achaja_core::cast::Speaker> {
    tauri::async_runtime::spawn_blocking(|| achaja_core::cast::discover(6)).await.unwrap_or_default()
}

/// Tests one account of the form (an empty password = the saved one).
#[tauri::command]
pub async fn mail_test(st: St<'_>, row: AccountForm) -> Result<Vec<(bool, String)>> {
    if row.host.trim().is_empty() || row.user.trim().is_empty() {
        return Ok(vec![(false, "Podaj serwer i login.".into())]);
    }
    let raw = achaja_core::config::peek_raw(&st.paths);
    let prev = raw["mail"]["accounts"].as_array().cloned().unwrap_or_default().into_iter().find(|a| format!("{}@{}", a["user"].as_str().unwrap_or(""), a["host"].as_str().unwrap_or("")) == row.key).unwrap_or(json!({}));
    let mut acct = prev.clone();
    for (k, v) in [("name", json!(row.name)), ("host", json!(row.host)), ("user", json!(row.user)), ("security", json!(row.security)), ("port", json!(row.port))] {
        acct[k] = v;
    }
    if !row.password.is_empty() {
        acct["password"] = json!(row.password);
    }
    let cfg = achaja_core::config::Config::new(json!({"language": raw["language"], "mail": {"accounts": [acct]}}));
    let Some(a) = achaja_core::mail::accounts::accounts(&cfg, true).into_iter().next() else { return Ok(vec![(false, "Podaj serwer i login.".into())]) };
    Ok(achaja_core::mail::test_account(&a).await)
}

#[tauri::command]
pub async fn ha_test(st: St<'_>, url: String, token: String) -> Result<(bool, String)> {
    Ok(achaja_core::homeassistant::test(&st.paths, &url, &token).await)
}

/// The claude.ai connectors (gmail, calendar) and their status, for the phone alerts settings.
#[tauri::command]
pub async fn phone_connectors(st: St<'_>, refresh: bool) -> Result<std::collections::BTreeMap<String, String>> {
    let paths = st.paths.clone();
    tauri::async_runtime::spawn_blocking(move || achaja_core::phone::connector::probe(&paths, refresh)).await.map_err(|e| AppError::msg(e.to_string()))
}

/// The phone alerts at a glance (the window's Today panel).
#[tauri::command]
pub fn phone_overview(st: St<'_>) -> achaja_core::phone::overview::Overview {
    achaja_core::phone::overview::overview(&st.paths)
}

#[tauri::command]
pub async fn ha_phones(st: St<'_>, url: String, token: String) -> Result<Vec<String>> {
    Ok(achaja_core::homeassistant::find_phones(&st.paths, &url, &token).await)
}

/// The model a language needs when it is not downloaded yet.
#[tauri::command]
pub fn model_status(st: St, language: String) -> Result<Option<String>> {
    let raw = achaja_core::config::peek_raw(&st.paths);
    Ok(settings::model_missing(&st.paths, &raw, &language))
}

/// Downloads a speech model; progress as "achaja://model-progress" events (0..1).
#[tauri::command]
pub async fn model_download(app: AppHandle, st: St<'_>, name: String) -> Result<()> {
    let progress = move |f: f64| {
        let _ = app.emit("achaja://model-progress", f);
    };
    achaja_core::stt::download_model(&st.paths, &name, &progress).await?;
    Ok(())
}
