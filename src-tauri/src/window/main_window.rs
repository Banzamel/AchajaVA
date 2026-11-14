//! The main window, created here rather than in tauri.conf.json so its WebView2 profile lives in
//! data\webview instead of %LOCALAPPDATA%. Frameless with the shared title bar; size and place
//! are remembered in state\app_window.json.

use achaja_core::json::{read_value, write_json};
use serde_json::json;
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

use super::native_frame;
use crate::host::AppState;

pub const LABEL: &str = "main";
const LAYOUT: u64 = 5; // a new layout ignores geometry saved for the old one (5: sections + Today panel)

pub fn create(app: &AppHandle, st: &AppState, visible: bool) -> tauri::Result<()> {
    let page = if visible { "index.html" } else { "index.html?minimized=1" };
    let w = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App(page.into()))
        .title("Achaja")
        .inner_size(1240.0, 800.0)
        .min_inner_size(420.0, 560.0)
        .decorations(false)
        .shadow(native_frame())
        .visible(false)
        .center()
        .data_directory(st.paths.webview())
        .build()?;
    let saved = read_value(&st.paths.state_file("app_window.json"));
    if saved["layout"].as_u64() == Some(LAYOUT) {
        if let (Some(x), Some(y), Some(width), Some(height)) = (saved["x"].as_i64(), saved["y"].as_i64(), saved["w"].as_u64(), saved["h"].as_u64()) {
            let on_screen = w.available_monitors().unwrap_or_default().iter().any(|m| {
                let (p, s) = (m.position(), m.size());
                x + 200 > i64::from(p.x) && x < i64::from(p.x) + i64::from(s.width) && y + 60 > i64::from(p.y) && y < i64::from(p.y) + i64::from(s.height)
            });
            if on_screen {
                let _ = w.set_size(PhysicalSize::new(width as u32, height as u32));
                let _ = w.set_position(PhysicalPosition::new(x as i32, y as i32));
            }
        }
    }
    // shown by the page once its first frame is drawn (no white flash); hidden when --minimized
    st.visible.store(visible, std::sync::atomic::Ordering::Relaxed);
    if !visible {
        let _ = w.hide();
    }
    Ok(())
}

pub fn save_geometry(app: &AppHandle, st: &AppState) {
    let Some(w) = app.get_webview_window(LABEL) else { return };
    if w.is_maximized().unwrap_or(false) || w.is_minimized().unwrap_or(false) {
        return;
    }
    if let (Ok(p), Ok(s)) = (w.outer_position(), w.outer_size()) {
        let _ = write_json(&st.paths.state_file("app_window.json"), &json!({"x": p.x, "y": p.y, "w": s.width, "h": s.height, "layout": LAYOUT}));
    }
}

pub fn show(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

pub fn hide(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.hide();
    }
}

pub fn is_visible(app: &AppHandle) -> bool {
    app.get_webview_window(LABEL).is_some_and(|w| w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false))
}
