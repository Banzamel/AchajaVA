//! The icon in the notification area: show the window, talk, pause or resume listening, quit.
//! The labels come from the window (translated) through set_texts.

use std::sync::Arc;

use achaja_core::listener::Request;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use crate::host::AppState;

pub const ID: &str = "main";

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Texts {
    pub show: String,
    pub talk: String,
    pub pause: String,
    pub resume: String,
    pub quit: String,
}

fn menu(app: &AppHandle, t: &Texts, paused: bool) -> tauri::Result<Menu<tauri::Wry>> {
    let show = MenuItem::with_id(app, "show", &t.show, true, None::<&str>)?;
    let talk = MenuItem::with_id(app, "talk", &t.talk, true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", if paused { &t.resume } else { &t.pause }, true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", &t.quit, true, None::<&str>)?;
    Menu::with_items(app, &[&show, &talk, &pause, &sep, &quit])
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let t = Texts { show: "Pokaż okno".into(), talk: "Mów".into(), pause: "Wstrzymaj nasłuch".into(), resume: "Wznów nasłuch".into(), quit: "Zakończ".into() };
    let mut b = TrayIconBuilder::with_id(ID).tooltip("Achaja").menu(&menu(app, &t, false)?).show_menu_on_left_click(false);
    if let Some(icon) = app.default_window_icon() {
        b = b.icon(icon.clone());
    }
    b.on_menu_event(|app, e| {
        let st = app.state::<Arc<AppState>>();
        match e.id().as_ref() {
            "show" => crate::window::main_window::show(app),
            "talk" => talk(&st),
            "pause" => {
                if let Some(b) = st.bridge() {
                    b.paused.store(!b.paused(), std::sync::atomic::Ordering::Relaxed);
                }
            }
            "quit" => crate::commands::assistant::quit_app(app, &st, false),
            _ => {}
        }
    })
    .on_tray_icon_event(|t, e| {
        if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
            crate::window::main_window::show(t.app_handle());
        }
    })
    .build(app)?;
    Ok(())
}

/// Talk: starts a recording, or sends the one in progress.
pub fn talk(st: &AppState) {
    if let Some(b) = st.bridge() {
        b.request(if b.state() == "recording" { Request::Send } else { Request::Record });
    }
}

pub fn set_texts(app: &AppHandle, t: &Texts, paused: bool) {
    let Some(tray) = app.tray_by_id(ID) else { return };
    if let Ok(m) = menu(app, t, paused) {
        let _ = tray.set_menu(Some(m));
    }
}
