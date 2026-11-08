//! Start-up: the data folder in the install folder (written there or nowhere), one running
//! Achaja (a second start shows the first one's window or settings), the data a Python version
//! left in the install folder, then the window, the tray and the assistant's threads.
//!   Achaja.exe              the window (--minimized: in the tray; --continue: resume the conversation)
//!   Achaja.exe --settings   the window on its Settings view (of the running Achaja, or a new start)

use std::sync::Arc;

use achaja_core::json::{now, read_value};
use achaja_core::paths::{ensure_dirs, paths};
use achaja_core::process::{pid_alive, process_name, single_instance};
use achaja_core::state::{self, APP, SETTINGS_FLAG, SHOW_FLAG};
use serde_json::json;
use tauri::App;

use crate::host::{self, AppState};
use crate::window::{fatal, main_window};

pub struct Start {
    pub state: Arc<AppState>,
    minimized: bool,
    resume: bool,
}

fn running_app() -> bool {
    let pid = read_value(&paths().state_file(APP))["pid"].as_u64().unwrap_or(0) as u32;
    pid != std::process::id() && pid_alive(pid) && process_name(pid).as_deref() == Some("achaja.exe")
}

pub fn prepare(args: &[String]) -> Option<Start> {
    let has = |f: &str| args.iter().any(|a| a == f);
    let p = paths().clone();
    if let Err(e) = ensure_dirs(&p) {
        fatal::show(&format!(
            "Achaja keeps its data next to the program, but cannot write there:\n{e}\n\nReinstall it into a folder you can write to, e.g. C:\\Banzamel\\Achaja."
        ));
        return None;
    }
    achaja_core::runtime::install_tls();
    crate::window::migrate::from_python(&p);
    let settings = has("--settings");
    if running_app() {
        // the running window shows itself (or its settings): no second listener
        state::write(&p, if settings { SETTINGS_FLAG } else { SHOW_FLAG }, &json!({ "at": now() }));
        return None;
    }
    if !single_instance("AchajaListener") {
        fatal::show("Achaja już działa. / Achaja is already running.");
        return None;
    }
    state::write(&p, APP, &json!({ "pid": std::process::id() }));
    // a settings window of an earlier test build had its own WebView2 profile
    let _ = std::fs::remove_dir_all(p.data.join("webview-settings"));
    let cfg = achaja_core::config::peek(&p);
    Some(Start { state: Arc::new(AppState::new(p, cfg, settings)), minimized: has("--minimized") && !settings, resume: has("--continue") })
}

pub fn setup(app: &mut App, start: &Start) -> tauri::Result<()> {
    let handle = app.handle().clone();
    let st = start.state.clone();
    if st.cfg().bool_or("app.tray", true) {
        crate::tray::create(&handle)?;
    }
    // the first start (no config.json yet) shows the wizard even when started --minimized
    main_window::create(&handle, &st, !start.minimized || !st.paths.config().exists())?;
    achaja_core::chat::trim(&st.paths, 400);
    if st.paths.config().exists() {
        // Claude's workspace in data\.claude is complete even before the session starts
        if let Err(e) = achaja_core::claude::workspace::write_files(&st.paths, &st.cfg()) {
            st.log(&format!("BLAD przygotowania .claude: {e}"));
        }
        host::listener::start(&st, start.resume);
    } else {
        // the first start after installing: the window opens on the settings wizard (the page
        // asks app_info); saving it writes config.json and the supervisor starts the listener
        st.first_run.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    host::screen::spawn(st.clone());
    host::supervisor::spawn(handle, st);
    Ok(())
}
