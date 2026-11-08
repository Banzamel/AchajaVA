//! Achaja's window: wiring of the core (achaja-core: listener, voice, Claude session, mail...) to
//! Tauri commands, the tray, the windows and the updater. No assistant logic lives here.

mod commands;
mod host;
mod launch;
mod tray;
mod updater;
mod window;

use commands::{agents, assistant, claude, plan, settings, system};

pub fn run(args: &[String]) {
    let Some(start) = launch::prepare(args) else { return };
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(start.state.clone())
        .setup(move |app| Ok(launch::setup(app, &start)?))
        .on_window_event(window::on_event)
        .invoke_handler(tauri::generate_handler![
            assistant::app_info, assistant::tick, assistant::request, assistant::set_paused, assistant::chat_history,
            assistant::toggle_console, assistant::repeat_last, assistant::restart_listener, assistant::set_terminal_open,
            assistant::quit, assistant::hide_window, assistant::tray_texts,
            agents::agents_list, agents::agent_log, agents::agent_stop, agents::agent_remove,
            plan::plan_items, plan::plan_detail, plan::plan_remove, plan::plan_run,
            claude::claude_account, claude::claude_usage, claude::claude_projects, claude::claude_sessions,
            claude::claude_delete, claude::claude_clean_preview, claude::claude_clean, claude::claude_open_dir,
            settings::settings_load, settings::settings_save, settings::audio_devices, settings::cast_discover,
            settings::mail_test, settings::ha_test, settings::ha_phones, settings::model_status, settings::model_download,
            system::edit_config, system::backup_now, system::open_backups, system::open_url,
            system::update_check, system::update_install,
        ])
        .run(tauri::generate_context!());
    if let Err(e) = result {
        window::fatal::show(&format!("Achaja could not start: {e}"));
    }
}
