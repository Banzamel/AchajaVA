//! A thread reading the hidden Claude console every moment: recording, the dictated text, the
//! activity line and choice dialogs (with the buffer's history while the Terminal tab is open).

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use achaja_core::console::screen::{markers, parse};

use super::state::AppState;

const HISTORY: i16 = 300;

pub fn spawn(st: Arc<AppState>) {
    std::thread::spawn(move || {
        while !st.quitting() {
            let marks = markers(&st.cfg());
            let screen = st.claude_pid().and_then(|pid| {
                let history = if st.term_open.load(Ordering::Relaxed) { HISTORY } else { 0 };
                achaja_core::console::read_screen(pid, history).ok().map(|lines| parse(&lines, &marks))
            });
            if let Ok(mut s) = st.screen.lock() {
                *s = screen;
            }
            let pause = if st.visible.load(Ordering::Relaxed) { 350 } else { 1500 };
            std::thread::sleep(Duration::from_millis(pause));
        }
    });
}
