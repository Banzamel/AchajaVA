//! Child processes and other programs: is a PID alive, its exe name, stopping a process tree,
//! starting Achaja's own commands without a window, and the environment of Claude Code.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;
pub const DETACHED_PROCESS: u32 = 0x0000_0008;
pub const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;

/// Markers of the Claude session that started us. Inherited, CLAUDE_CODE_CHILD_SESSION and the
/// others switch off conversation saving in the claude we start (and --resume then fails).
const CLAUDE_MARKERS: &[&str] = &[
    "CLAUDECODE", "CLAUDE_PID", "CLAUDE_CODE_CHILD_SESSION", "CLAUDE_CODE_ENTRYPOINT", "CLAUDE_CODE_SESSION_ID",
    "CLAUDE_CODE_SESSION_ATTENDED", "CLAUDE_CODE_BRIDGE_SESSION_ID", "CLAUDE_CODE_MESSAGING_SOCKET",
    "CLAUDE_CODE_MESSAGING_TOKEN", "CLAUDE_CODE_SSE_PORT",
];

pub fn clean_env() -> Vec<(String, String)> {
    std::env::vars().filter(|(k, _)| !CLAUDE_MARKERS.contains(&k.to_uppercase().as_str())).collect()
}

/// A command with no console window (a hidden child of a windowless app).
pub fn hidden(program: impl AsRef<std::ffi::OsStr>) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// A claude command with a clean environment (no markers of the session that started us).
pub fn claude_command(claude: &Path) -> Command {
    let mut cmd = hidden(claude);
    cmd.env_clear().envs(clean_env());
    cmd
}

/// Starts Achaja.exe with the arguments in the background (speech, mail watcher, jobs) and
/// returns its PID; nothing waits for it.
pub fn spawn_self(args: &[&str], cwd: &Path) -> std::io::Result<u32> {
    let exe = crate::paths::paths().exe.clone();
    #[allow(unused_mut)]
    let mut cmd = Command::new(exe);
    cmd.args(args).current_dir(cwd).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
    }
    cmd.spawn().map(|c| c.id())
}

/// The claude command from PATH (claude.exe of the native installer or claude.cmd of npm).
pub fn find_claude() -> Option<PathBuf> {
    which::which("claude").ok()
}

/// Stops a process with all its children (taskkill /T /F).
pub fn kill_tree(pid: u32) {
    let _ = hidden("taskkill").args(["/PID", &pid.to_string(), "/T", "/F"]).output();
}

#[cfg(windows)]
pub fn pid_alive(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    if pid == 0 {
        return false;
    }
    // SAFETY: plain Win32 calls on a handle we own and close.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return false;
        }
        let mut code = 0u32;
        let ok = GetExitCodeProcess(handle, &mut code);
        CloseHandle(handle);
        ok != 0 && code == 259 // STILL_ACTIVE
    }
}

#[cfg(not(windows))]
pub fn pid_alive(pid: u32) -> bool {
    pid != 0 && Path::new(&format!("/proc/{pid}")).exists()
}

/// Lower-case exe file name of a process ("claude.exe"), None when it is gone.
#[cfg(windows)]
pub fn process_name(pid: u32) -> Option<String> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION};
    // SAFETY: the buffer outlives the call; the handle is closed.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }
        let mut buf = [0u16; 1024];
        let mut size = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size);
        CloseHandle(handle);
        if ok == 0 {
            return None;
        }
        let full = String::from_utf16_lossy(&buf[..size as usize]);
        Path::new(&full).file_name().map(|n| n.to_string_lossy().to_lowercase())
    }
}

#[cfg(not(windows))]
pub fn process_name(_pid: u32) -> Option<String> {
    None
}

/// Held for the life of the process: true when this is the only instance with that name.
#[cfg(windows)]
pub fn single_instance(name: &str) -> bool {
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use windows_sys::Win32::System::Threading::CreateMutexW;
    let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: the name is NUL-terminated; the handle is deliberately leaked (released at exit).
    unsafe {
        let handle = CreateMutexW(std::ptr::null(), 0, wide.as_ptr());
        !handle.is_null() && GetLastError() != ERROR_ALREADY_EXISTS
    }
}

#[cfg(not(windows))]
pub fn single_instance(_name: &str) -> bool {
    true
}
