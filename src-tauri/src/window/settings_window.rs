//! The settings window (index.html#settings): from the menu of the main window, from the
//! "Achaja – ustawienia" shortcut (Achaja.exe --settings), or on its own when Achaja is not
//! running — then with a separate WebView2 profile, so a main window started meanwhile does not
//! share it.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use super::native_frame;
use crate::host::AppState;

pub const LABEL: &str = "settings";

pub fn open(app: &AppHandle, st: &AppState) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.show();
        let _ = w.unminimize();
        return w.set_focus();
    }
    let editing = st.paths.config().exists() && !st.first_run.load(std::sync::atomic::Ordering::Relaxed);
    let (w, h, min_w, min_h) = if editing { (900.0, 680.0, 780.0, 600.0) } else { (780.0, 720.0, 700.0, 640.0) };
    let data = if st.settings_only { st.paths.data.join("webview-settings") } else { st.paths.webview() };
    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html#settings".into()))
        .title("Achaja — ustawienia")
        .inner_size(w, h)
        .min_inner_size(min_w, min_h)
        .decorations(false)
        .shadow(native_frame())
        .visible(false)
        .center()
        .data_directory(data)
        .build()?;
    Ok(())
}
