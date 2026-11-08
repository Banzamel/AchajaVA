//! The listener thread: started at launch, again after a settings change (attaching to the same
//! Claude session) and when the session was closed (continuing the same conversation).

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use achaja_core::listener::{self, Bridge};

use super::state::AppState;

pub fn start(st: &Arc<AppState>, resume: bool) {
    if st.no_listener {
        st.log("Nasluch wylaczony (ACHAJA_NO_LISTENER).");
        return;
    }
    let paused = st.bridge().map(|b| b.paused()).unwrap_or(false);
    let hidden = st.cfg().bool_or("app.hide_console", true);
    let feed = st.clone();
    let bridge = Arc::new(Bridge::new(hidden, paused, Arc::new(move |line: String| feed.log_line(line))));
    if let Ok(mut b) = st.bridge.write() {
        *b = Some(bridge.clone());
    }
    if let Ok(mut t) = st.last_start.lock() {
        *t = achaja_core::json::now();
    }
    let paths = st.paths.clone();
    let handle = std::thread::spawn(move || match listener::run(&paths, &bridge, resume) {
        Ok("closed") => bridge.set_state("closed"),
        Ok(_) => {}
        Err(e) => {
            bridge.log(&format!("BLAD: {e}"));
            bridge.set_state("error");
        }
    });
    if let Ok(mut l) = st.listener.lock() {
        *l = Some(handle);
    }
}

pub fn stop(st: &AppState) {
    if let Some(b) = st.bridge() {
        b.stop.store(true, Ordering::Relaxed);
    }
    let handle = st.listener.lock().ok().and_then(|mut l| l.take());
    if let Some(h) = handle {
        let deadline = Instant::now() + Duration::from_secs(3);
        while !h.is_finished() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

/// Attaches again to the same Claude session when it still runs.
pub fn restart(st: &Arc<AppState>) {
    stop(st);
    start(st, true);
}
