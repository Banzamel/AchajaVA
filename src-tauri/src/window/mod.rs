//! Achaja's one window (settings are a view inside it), a start-up error box, the move of the
//! Python version's data, and what the X button does.

pub mod fatal;
pub mod main_window;
pub mod migrate;

use tauri::{AppHandle, Emitter, Manager, Window, WindowEvent};

use crate::host::AppState;

/// Windows 11 gives the frameless window its border, shadow and rounded corners through Tauri's
/// window shadow. On Windows 10 that frame has no top edge, so it stays off and the page draws a
/// 1px border on all four sides instead (src/shared/ui/titlebar).
#[cfg(windows)]
pub fn native_frame() -> bool {
    windows_version::OsVersion::current().build >= 22000
}

#[cfg(not(windows))]
pub fn native_frame() -> bool {
    false
}

/// The main window's X hides it to the tray (app.close_to_tray) or asks the page to confirm
/// quitting.
pub fn on_event(win: &Window, event: &WindowEvent) {
    let app = win.app_handle();
    let st = app.state::<std::sync::Arc<AppState>>();
    match (win.label(), event) {
        (main_window::LABEL, WindowEvent::CloseRequested { api, .. }) => {
            if st.quitting() {
                return;
            }
            api.prevent_close();
            main_window::save_geometry(app, &st);
            let tray = st.cfg().bool_or("app.tray", true) && app.tray_by_id(crate::tray::ID).is_some();
            if tray && st.cfg().bool_or("app.close_to_tray", true) {
                main_window::hide(app);
            } else {
                let _ = app.emit_to(main_window::LABEL, "achaja://quit-ask", ());
            }
        }
        _ => {}
    }
}

/// Shows the window on its Settings view (the menu, the tray, `Achaja.exe --settings`).
pub fn open_settings(app: &AppHandle) {
    main_window::show(app);
    let _ = app.emit_to(main_window::LABEL, "achaja://open-settings", ());
}
