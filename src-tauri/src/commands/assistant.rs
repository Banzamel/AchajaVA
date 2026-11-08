//! The main window's assistant: one snapshot every few hundred milliseconds (the listener's
//! state, the Claude console, the session's work state, new log lines, notes and chat messages)
//! and the actions of its buttons, menu and tray.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use achaja_core::chat::{self, ChatLine};
use achaja_core::console::screen::Screen;
use achaja_core::error::Result;
use achaja_core::listener::Request;
use achaja_core::state;
use serde::Serialize;
use tauri::{AppHandle, State};

use crate::host::state::{FeedItem, UpdateView};
use crate::host::AppState;

type St<'a> = State<'a, Arc<AppState>>;

#[derive(Serialize)]
pub struct AppInfo {
    version: String,
    language: String,
    theme: String,
    hint_words: HintWords,
    data: String,
}

#[derive(Serialize)]
pub struct HintWords {
    end: String,
    cancel: String,
    clear: String,
}

#[derive(Serialize)]
pub struct Tick {
    state: String,
    paused: bool,
    heard: String,
    problem: Option<String>,
    screen: Option<Screen>,
    work: String,
    work_since: f64,
    work_kind: String,
    speaking: bool,
    feed: Vec<FeedItem>,
    chat: Vec<ChatLine>,
    chat_offset: u64,
    update: UpdateView,
    tray: bool,
}

#[tauri::command]
pub fn app_info(st: St) -> AppInfo {
    let cfg = st.cfg();
    AppInfo {
        version: achaja_core::VERSION.into(),
        language: cfg.language(),
        theme: cfg.str_or("app.theme", "auto"),
        hint_words: HintWords { end: cfg.first("wake.end", "wykonaj"), cancel: cfg.first("wake.cancel", "anuluj"), clear: cfg.first("wake.clear", "nowa rozmowa") },
        data: st.paths.data.to_string_lossy().into_owned(),
    }
}

#[tauri::command]
pub fn tick(app: AppHandle, st: St, feed_after: u64, chat_offset: u64) -> Tick {
    let bridge = st.bridge();
    let term_open = st.term_open.load(Ordering::Relaxed);
    let mut screen = st.claude_pid().and_then(|_| st.screen.lock().ok().and_then(|s| s.clone()));
    if let (Some(s), false) = (screen.as_mut(), term_open) {
        s.lines.clear(); // the lines only for the Terminal tab
    }
    let (work, work_since, work_kind) = state::work(&st.paths);
    let (chat, chat_offset) = chat::read_since(&st.paths, chat_offset);
    Tick {
        state: bridge.as_ref().map(|b| b.state()).unwrap_or_else(|| if st.no_listener { "paused".into() } else { "start".into() }),
        paused: bridge.as_ref().is_some_and(|b| b.paused()),
        heard: bridge.as_ref().map(|b| b.heard()).unwrap_or_default(),
        problem: bridge.as_ref().and_then(|b| b.take_problem()),
        screen,
        work,
        work_since,
        work_kind,
        speaking: state::speaking(&st.paths),
        feed: st.feed.lock().map(|f| f.since(feed_after)).unwrap_or_default(),
        chat,
        chat_offset,
        update: st.update.lock().map(|u| u.clone()).unwrap_or_default(),
        tray: app.tray_by_id(crate::tray::ID).is_some(),
    }
}

/// The conversation's last messages and the offset to read new ones from.
#[tauri::command]
pub fn chat_history(st: St) -> (Vec<ChatLine>, u64) {
    chat::last(&st.paths, 60)
}

/// record | send | cancel | clear | talk | type:<text> | keys:<k1,k2> | say:<text>
#[tauri::command]
pub fn request(st: St, action: String) {
    let Some(b) = st.bridge() else { return };
    let req = match action.as_str() {
        "record" => Request::Record,
        "send" => Request::Send,
        "cancel" => Request::Cancel,
        "clear" => Request::Clear,
        "talk" => return crate::tray::talk(&st),
        a if a.starts_with("type:") => Request::Type(a[5..].to_string()),
        a if a.starts_with("keys:") => Request::Keys(a[5..].split(',').map(str::to_string).collect()),
        a if a.starts_with("say:") => return achaja_core::voice::speak::say(&st.paths, &a[4..]),
        _ => return,
    };
    b.request(req);
}

#[tauri::command]
pub fn set_paused(st: St, paused: bool) {
    if let Some(b) = st.bridge() {
        b.paused.store(paused, Ordering::Relaxed);
    }
}

#[tauri::command]
pub fn set_terminal_open(st: St, open: bool) {
    st.term_open.store(open, Ordering::Relaxed);
}

#[tauri::command]
pub fn toggle_console(st: St) {
    if let Some(pid) = st.claude_pid() {
        achaja_core::console::set_visible(pid, !achaja_core::console::is_visible(pid));
    }
}

#[tauri::command]
pub fn repeat_last(st: St) {
    if let Some(text) = achaja_core::json::read_text(&st.paths.state_file(state::LAST_VOICE)) {
        achaja_core::voice::speak::say(&st.paths, &text);
    }
}

#[tauri::command]
pub fn restart_listener(st: St) {
    crate::host::listener::restart(&st);
}

#[tauri::command]
pub fn hide_window(app: AppHandle) {
    crate::window::main_window::hide(&app);
}

#[tauri::command]
pub fn tray_texts(app: AppHandle, st: St, texts: crate::tray::Texts) {
    crate::tray::set_texts(&app, &texts, st.bridge().is_some_and(|b| b.paused()));
}

/// install_now: after closing, the downloaded version is installed and Achaja starts again.
#[tauri::command]
pub fn quit(app: AppHandle, st: St, install_now: bool) -> Result<()> {
    quit_app(&app, &st, install_now);
    Ok(())
}

pub fn quit_app(app: &AppHandle, st: &AppState, install_now: bool) {
    crate::window::main_window::save_geometry(app, st);
    if crate::updater::ready() && (install_now || st.cfg().bool_or("app.update_install_on_quit", true)) {
        crate::updater::install_now(app, st, install_now); // exits the process
    }
    crate::host::shutdown::stop_everything(st);
    app.exit(0);
}
