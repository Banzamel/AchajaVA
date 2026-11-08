//! The console of another process (the hidden Claude Code window), without its focus: key
//! presses go straight into its input buffer (WriteConsoleInputW) and the screen is read with
//! ReadConsoleOutputCharacterW. A process can be attached to one console at a time, so every
//! access holds a lock, and our own standard handles are restored afterwards.

use std::sync::Mutex;

use windows_sys::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, HWND, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING};
use windows_sys::Win32::System::Console::{
    AttachConsole, FreeConsole, GetConsoleScreenBufferInfo, GetConsoleWindow, GetStdHandle, ReadConsoleOutputCharacterW,
    SetConsoleCtrlHandler, SetStdHandle, WriteConsoleInputW, CONSOLE_SCREEN_BUFFER_INFO, COORD, INPUT_RECORD, KEY_EVENT,
    STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{IsIconic, IsWindowVisible, SetForegroundWindow, ShowWindow, SW_HIDE, SW_RESTORE, SW_SHOW};

use super::keys::{expand, Key};
use crate::error::{AppError, Result};

static LOCK: Mutex<()> = Mutex::new(());
const STD: [u32; 3] = [STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE];

fn last_error(what: &str) -> AppError {
    AppError::Console(format!("{what}: {}", std::io::Error::last_os_error()))
}

/// Runs `f` while attached to the console of `pid`.
fn attached<T>(pid: u32, f: impl FnOnce() -> Result<T>) -> Result<T> {
    let _guard = LOCK.lock().map_err(|_| AppError::Console("console lock poisoned".into()))?;
    // SAFETY: Win32 console calls; the saved handles are put back before the lock is released.
    unsafe {
        let saved: Vec<HANDLE> = STD.iter().map(|n| GetStdHandle(*n)).collect();
        FreeConsole();
        if AttachConsole(pid) == 0 {
            let e = last_error(&format!("AttachConsole({pid})"));
            for (n, h) in STD.iter().zip(&saved) {
                SetStdHandle(*n, *h);
            }
            return Err(e);
        }
        SetConsoleCtrlHandler(None, 1); // Ctrl+C in Claude's window must not stop us
        let result = f();
        FreeConsole();
        for (n, h) in STD.iter().zip(&saved) {
            SetStdHandle(*n, *h);
        }
        result
    }
}

fn open(name: &str) -> Result<HANDLE> {
    let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: NUL-terminated name; the handle is closed by the caller.
    let h = unsafe {
        CreateFileW(wide.as_ptr(), GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, std::ptr::null(), OPEN_EXISTING, 0, std::ptr::null_mut())
    };
    if h == INVALID_HANDLE_VALUE || h.is_null() {
        return Err(last_error(name));
    }
    Ok(h)
}

fn record(k: &Key, down: bool) -> INPUT_RECORD {
    // SAFETY: INPUT_RECORD is plain data; zeroed is a valid starting value.
    let mut rec: INPUT_RECORD = unsafe { std::mem::zeroed() };
    rec.EventType = KEY_EVENT as u16;
    rec.Event.KeyEvent.bKeyDown = i32::from(down);
    rec.Event.KeyEvent.wRepeatCount = 1;
    rec.Event.KeyEvent.wVirtualKeyCode = k.vk;
    rec.Event.KeyEvent.wVirtualScanCode = k.scan;
    rec.Event.KeyEvent.uChar.UnicodeChar = k.ch;
    rec.Event.KeyEvent.dwControlKeyState = k.ctrl;
    rec
}

pub fn write_keys(pid: u32, keys: &[String]) -> Result<()> {
    let records: Vec<INPUT_RECORD> = expand(keys).iter().flat_map(|k| [record(k, true), record(k, false)]).collect();
    if records.is_empty() {
        return Ok(());
    }
    attached(pid, || {
        let h = open("CONIN$")?;
        let mut written = 0u32;
        // SAFETY: the records live for the call; the handle is closed after.
        let ok = unsafe { WriteConsoleInputW(h, records.as_ptr(), records.len() as u32, &mut written) };
        unsafe { CloseHandle(h) };
        if ok == 0 {
            return Err(last_error("WriteConsoleInputW"));
        }
        Ok(())
    })
}

/// The visible lines of the console window, plus `history` lines of the buffer above it.
pub fn read_screen(pid: u32, history: i16) -> Result<Vec<String>> {
    attached(pid, || {
        let h = open("CONOUT$")?;
        let result = (|| {
            // SAFETY: plain data out-parameter.
            let mut info: CONSOLE_SCREEN_BUFFER_INFO = unsafe { std::mem::zeroed() };
            if unsafe { GetConsoleScreenBufferInfo(h, &mut info) } == 0 {
                return Err(last_error("GetConsoleScreenBufferInfo"));
            }
            let win = info.srWindow;
            let width = (win.Right - win.Left + 1).max(1) as usize;
            let mut buf = vec![0u16; width];
            let mut lines = Vec::new();
            for y in (win.Top - history).max(0)..=win.Bottom {
                let mut read = 0u32;
                // SAFETY: the buffer holds `width` characters.
                unsafe { ReadConsoleOutputCharacterW(h, buf.as_mut_ptr(), width as u32, COORD { X: win.Left, Y: y }, &mut read) };
                lines.push(String::from_utf16_lossy(&buf[..read as usize]).trim_end().to_string());
            }
            Ok(lines)
        })();
        unsafe { CloseHandle(h) };
        result
    })
}

fn console_window(pid: u32) -> Option<HWND> {
    // SAFETY: a query of the attached console.
    attached(pid, || Ok(unsafe { GetConsoleWindow() })).ok().filter(|h| !h.is_null())
}

/// Shows or hides Claude's console window (may not work when Windows Terminal hosts consoles).
pub fn set_visible(pid: u32, visible: bool) -> bool {
    let Some(hwnd) = console_window(pid) else { return false };
    // SAFETY: plain window calls on a window handle we just got.
    unsafe {
        if visible {
            ShowWindow(hwnd, if IsIconic(hwnd) != 0 { SW_RESTORE } else { SW_SHOW });
            SetForegroundWindow(hwnd);
        } else {
            ShowWindow(hwnd, SW_HIDE);
        }
    }
    true
}

pub fn is_visible(pid: u32) -> bool {
    // SAFETY: a query on a window handle.
    console_window(pid).is_some_and(|h| unsafe { IsWindowVisible(h) } != 0)
}
