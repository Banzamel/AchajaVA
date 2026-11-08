//! Self-update from GitHub Releases (signed latest.json): checked 20 s after the start and every
//! 15 minutes, downloaded in the background (app.update_auto_download), installed when Achaja
//! quits (app.update_install_on_quit) or at once with "Install now" — then it starts again.
//! The installer keeps settings and data.

use std::sync::{Arc, Mutex};

use tauri::AppHandle;
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::host::AppState;

static READY: Mutex<Option<(Update, Vec<u8>)>> = Mutex::new(None);
static BUSY: Mutex<bool> = Mutex::new(false);

fn set(st: &AppState, f: impl FnOnce(&mut crate::host::state::UpdateView)) {
    if let Ok(mut u) = st.update.lock() {
        f(&mut u);
    }
}

pub fn ready() -> bool {
    READY.lock().map(|r| r.is_some()).unwrap_or(false)
}

/// Checks (and downloads when allowed); with `install` installs right after, restarting Achaja.
pub fn check(app: AppHandle, st: Arc<AppState>, manual: bool, install: bool) {
    if install && ready() {
        return install_now(&app, &st, true);
    }
    {
        let Ok(mut busy) = BUSY.lock() else { return };
        if *busy {
            return;
        }
        *busy = true;
    }
    tauri::async_runtime::spawn(async move {
        set(&st, |u| {
            u.phase = "checking".into();
            u.manual = manual;
        });
        let result = run(&app, &st, manual || install).await;
        if let Err(e) = result {
            st.log(&format!("Aktualizacja: {e}"));
            set(&st, |u| {
                u.phase = if manual { "error".into() } else { String::new() };
                u.error = e;
            });
        }
        if let Ok(mut b) = BUSY.lock() {
            *b = false;
        }
        if install && ready() {
            install_now(&app, &st, true);
        }
    });
}

async fn run(app: &AppHandle, st: &AppState, download: bool) -> Result<(), String> {
    if cfg!(debug_assertions) {
        set(st, |u| u.phase = if u.manual { "latest".into() } else { String::new() });
        return Ok(()); // development builds do not update themselves
    }
    let updater = app.updater().map_err(|e| e.to_string())?;
    let Some(update) = updater.check().await.map_err(|e| e.to_string())? else {
        set(st, |u| u.phase = if u.manual { "latest".into() } else { String::new() });
        return Ok(());
    };
    let version = update.version.clone();
    achaja_core::update::save_notes(&st.paths, &version, update.body.as_deref().unwrap_or(""));
    if !(download || st.cfg().bool_or("app.update_auto_download", true)) {
        set(st, |u| {
            u.phase = "available".into();
            u.version = version;
        });
        return Ok(());
    }
    let total = std::sync::atomic::AtomicU64::new(0);
    let got = std::sync::atomic::AtomicU64::new(0);
    let bytes = update
        .download(
            |chunk, len| {
                use std::sync::atomic::Ordering::Relaxed;
                if let Some(l) = len {
                    total.store(l, Relaxed);
                }
                let done = got.fetch_add(chunk as u64, Relaxed) + chunk as u64;
                let t = total.load(Relaxed).max(1);
                set(st, |u| {
                    u.phase = "downloading".into();
                    u.version = version.clone();
                    u.pct = (done * 100 / t) as u32;
                });
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())?;
    st.log(&format!("Pobrano nowa wersje {version}."));
    if let Ok(mut r) = READY.lock() {
        *r = Some((update, bytes));
    }
    set(st, |u| {
        u.phase = "ready".into();
        u.version = version;
    });
    Ok(())
}

/// Closes everything and runs the installer (it starts Achaja again when `restart`).
pub fn install_now(app: &AppHandle, st: &AppState, restart: bool) {
    let Some((update, bytes)) = READY.lock().ok().and_then(|mut r| r.take()) else { return };
    crate::window::main_window::save_geometry(app, st);
    crate::host::shutdown::stop_everything(st);
    let update = update.restart_after_install(restart);
    if let Err(e) = update.install(bytes) {
        st.log(&format!("Instalacja aktualizacji: {e}"));
    }
}
