//! A start-up error the user must see: there is no window yet, so a plain message box.

#[cfg(windows)]
pub fn show(text: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
    let wide = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let (title, body) = (wide("Achaja"), wide(text));
    // SAFETY: both strings are NUL-terminated UTF-16 buffers alive for the call.
    unsafe { MessageBoxW(std::ptr::null_mut(), body.as_ptr(), title.as_ptr(), MB_OK | MB_ICONERROR) };
}

#[cfg(not(windows))]
pub fn show(text: &str) {
    eprintln!("{text}");
}
