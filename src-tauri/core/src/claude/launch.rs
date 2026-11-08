//! Starting the Achaja session: claude in a new console that starts hidden (no flash of a
//! terminal window), in the data folder, with the markers its hooks recognize the session by
//! and the Home Assistant token for .mcp.json.

use crate::config::Config;
use crate::error::{AppError, Result};
use crate::json::read_text;
use crate::paths::Paths;
use crate::process::{clean_env, find_claude};

use super::args::{command_line, session_args};

/// The session's environment: ours without the markers of a parent Claude session.
pub fn session_env(paths: &Paths, hidden: bool) -> Vec<(String, String)> {
    let mut env = clean_env();
    env.push(("ACHAJA_SESSION".into(), "1".into()));
    env.push(("ACHAJA_EXE".into(), paths.exe.to_string_lossy().into_owned()));
    env.push(("ACHAJA_DATA".into(), paths.data.to_string_lossy().into_owned()));
    if hidden {
        env.push(("ACHAJA_APP".into(), "1".into()));
    }
    if let Some(token) = read_text(&paths.ha_token()).map(|t| t.trim().to_string()).filter(|t| !t.is_empty()) {
        env.push(("HA_TOKEN".into(), token)); // only this session gets it, for ${HA_TOKEN} in .mcp.json
    }
    env
}

/// Starts the session and returns claude's PID.
pub fn launch(paths: &Paths, cfg: &Config, resume: bool, hidden: bool) -> Result<u32> {
    let claude = find_claude().ok_or_else(|| AppError::msg("Nie znaleziono 'claude' w PATH."))?;
    let (app, line) = command_line(&claude, &session_args(cfg, resume));
    spawn_console(app.as_deref(), &line, &paths.data, &session_env(paths, hidden), hidden)
}

#[cfg(windows)]
fn spawn_console(app: Option<&str>, line: &str, cwd: &std::path::Path, env: &[(String, String)], hidden: bool) -> Result<u32> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        CreateProcessW, CREATE_NEW_CONSOLE, CREATE_UNICODE_ENVIRONMENT, PROCESS_INFORMATION, STARTF_USESHOWWINDOW, STARTUPINFOW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{SW_HIDE, SW_SHOWNORMAL};

    let wide = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let app_w = app.map(wide);
    let mut line_w = wide(line);
    let cwd_w = wide(&cwd.to_string_lossy());
    let mut block: Vec<u16> = Vec::new();
    for (k, v) in env {
        block.extend(format!("{k}={v}").encode_utf16());
        block.push(0);
    }
    block.push(0);
    // SAFETY: zeroed plain structs; every buffer outlives the call; handles are closed.
    unsafe {
        let mut si: STARTUPINFOW = std::mem::zeroed();
        si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        si.dwFlags = STARTF_USESHOWWINDOW;
        si.wShowWindow = (if hidden { SW_HIDE } else { SW_SHOWNORMAL }) as u16;
        let mut pi: PROCESS_INFORMATION = std::mem::zeroed();
        let ok = CreateProcessW(
            app_w.as_ref().map_or(std::ptr::null(), |a| a.as_ptr()),
            line_w.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            CREATE_NEW_CONSOLE | CREATE_UNICODE_ENVIRONMENT,
            block.as_ptr() as *const std::ffi::c_void,
            cwd_w.as_ptr(),
            &si,
            &mut pi,
        );
        if ok == 0 {
            return Err(AppError::Console(format!("claude: {}", std::io::Error::last_os_error())));
        }
        CloseHandle(pi.hThread);
        CloseHandle(pi.hProcess);
        Ok(pi.dwProcessId)
    }
}

#[cfg(not(windows))]
fn spawn_console(_app: Option<&str>, _line: &str, _cwd: &std::path::Path, _env: &[(String, String)], _hidden: bool) -> Result<u32> {
    Err(AppError::Console("the Claude console needs Windows".into()))
}
