//! The hidden Claude Code console: key presses, screen reading and visibility (Windows), and
//! the pure reading of what the screen shows.

pub mod keys;
pub mod screen;
#[cfg(windows)]
mod win;

#[cfg(windows)]
pub use win::{is_visible, read_screen, set_visible, write_keys};

#[cfg(not(windows))]
mod other {
    use crate::error::{AppError, Result};

    pub fn write_keys(_pid: u32, _keys: &[String]) -> Result<()> {
        Err(AppError::Console("console access needs Windows".into()))
    }
    pub fn read_screen(_pid: u32, _history: i16) -> Result<Vec<String>> {
        Err(AppError::Console("console access needs Windows".into()))
    }
    pub fn set_visible(_pid: u32, _visible: bool) -> bool {
        false
    }
    pub fn is_visible(_pid: u32) -> bool {
        false
    }
}

#[cfg(not(windows))]
pub use other::{is_visible, read_screen, set_visible, write_keys};
