//! A thread watching what happens around the window: another start asking to show it (or its
//! settings), config.json saved by the Settings view or an editor (the listener restarts), the
//! console window when Claude asks for approval, a closed Claude session (it comes back with the
//! same conversation), an update asked for by voice, and the update checks.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use achaja_core::json::{mtime, now};
use achaja_core::state::{self, SETTINGS_FLAG, SHOW_FLAG, UPDATE_REQUEST};
use tauri::AppHandle;

use super::state::AppState;

struct Memory {
    config_mtime: f64,
    hidden_once: u32,
    auto_shown: bool,
    next_update: f64,
    announced: bool,
    started: f64,
}

pub fn spawn(app: AppHandle, st: Arc<AppState>) {
    std::thread::spawn(move || {
        let mut m = Memory { config_mtime: mtime(&st.paths.config()), hidden_once: 0, auto_shown: false, next_update: now() + 20.0, announced: false, started: now() };
        while !st.quitting() {
            tick(&app, &st, &mut m);
            std::thread::sleep(Duration::from_millis(400));
        }
    });
}

fn tick(app: &AppHandle, st: &Arc<AppState>, m: &mut Memory) {
    let p = &st.paths;
    if p.state_file(SHOW_FLAG).exists() {
        state::clear(p, SHOW_FLAG);
        crate::window::main_window::show(app);
    }
    if p.state_file(SETTINGS_FLAG).exists() {
        state::clear(p, SETTINGS_FLAG);
        crate::window::open_settings(app);
    }
    let mt = mtime(&p.config());
    if mt != m.config_mtime {
        m.config_mtime = mt;
        if let Ok(cfg) = achaja_core::config::load(p) {
            if let Ok(mut c) = st.cfg.write() {
                *c = cfg.clone();
            }
            st.note("@config_changed");
            restart_mail(st, &cfg);
            super::listener::restart(st);
        }
    }
    let cfg = st.cfg();
    let bridge = st.bridge();
    if let (Some(pid), Some(b)) = (st.claude_pid(), bridge.as_ref()) {
        // a session from the previous start may have been visible: hidden once after attaching
        if b.hidden && m.hidden_once != pid {
            m.hidden_once = pid;
            achaja_core::console::set_visible(pid, false);
        }
        let (work, _, kind) = state::work(p);
        let waiting = work == "waiting" && kind == "permission_prompt";
        if waiting && !m.auto_shown && cfg.bool_or("app.show_console_on_prompt", false) {
            if !achaja_core::console::is_visible(pid) {
                achaja_core::console::set_visible(pid, true);
                m.auto_shown = true;
                st.note("@wait_note");
            }
        } else if m.auto_shown && !waiting && b.hidden {
            achaja_core::console::set_visible(pid, false);
            m.auto_shown = false;
        }
    }
    // a closed or killed session comes back, but not more often than every 30 s
    let closed = bridge.as_ref().is_some_and(|b| b.state() == "closed");
    let since_start = now() - st.last_start.lock().map(|t| *t).unwrap_or(0.0);
    if closed && cfg.bool_or("app.restart_closed_session", true) && since_start > 30.0 {
        st.note("@st_closed");
        super::listener::start(st, true);
    }
    if p.state_file(UPDATE_REQUEST).exists() {
        state::clear(p, UPDATE_REQUEST);
        crate::updater::check(app.clone(), st.clone(), true, true);
    }
    if !m.announced && now() - m.started > 4.0 {
        m.announced = true;
        if let Some((spoken, note)) = achaja_core::update::whats_new(p, st.lang()) {
            st.note(&note);
            achaja_core::voice::speak::say(p, &spoken);
        }
    }
    if cfg.bool_or("app.update_check", true) && now() >= m.next_update {
        m.next_update = now() + 15.0 * 60.0;
        crate::updater::check(app.clone(), st.clone(), false, false);
    }
    st.visible.store(crate::window::main_window::is_visible(app), Ordering::Relaxed);
}

/// Settings turn mail on or off: the watcher follows.
fn restart_mail(st: &AppState, cfg: &achaja_core::config::Config) {
    let p = &st.paths;
    achaja_core::mail::stop_watch(p);
    if cfg.bool_or("mail.enabled", false) && !achaja_core::mail::accounts::accounts(cfg, false).is_empty() {
        let _ = achaja_core::process::spawn_self(&["mail", "watch"], &p.data);
    }
}
