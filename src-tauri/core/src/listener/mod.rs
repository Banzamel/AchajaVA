//! The listener: wake word -> Claude Code's own dictation (voice tap mode) -> end word -> send.
//! Vosk recognizes ONLY the control words (small model, offline); the prompt itself is
//! transcribed by Claude Code. Keys go straight into the hidden Claude console, so it never
//! needs focus. It runs in a thread of the window and talks to it through the Bridge.

mod control;
mod replies;
mod run;
mod session;
mod ticks;
pub mod wake;

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

pub use run::run;

/// What the window asks the listener to do (buttons, the chat box, the Terminal tab's keys).
#[derive(Debug, Clone)]
pub enum Request {
    Record,
    Send,
    Cancel,
    Clear,
    Type(String),
    Keys(Vec<String>),
}

pub type Sink = std::sync::Arc<dyn Fn(String) + Send + Sync>;

pub struct Bridge {
    pub stop: AtomicBool,
    pub paused: AtomicBool,
    pub hidden: bool,
    pub pid: AtomicU32,
    /// "start", "idle", "recording", "wake", "closed", "error".
    pub state: Mutex<String>,
    /// What Vosk hears while recording (shown live until Claude's box has the text).
    pub heard: Mutex<String>,
    /// "dialog" / "rec_failed": the window then shows the Terminal tab.
    pub problem: Mutex<Option<String>>,
    requests: Mutex<VecDeque<Request>>,
    sink: Sink,
}

impl Bridge {
    pub fn new(hidden: bool, paused: bool, sink: Sink) -> Self {
        Bridge {
            stop: AtomicBool::new(false),
            paused: AtomicBool::new(paused),
            hidden,
            pid: AtomicU32::new(0),
            state: Mutex::new("start".into()),
            heard: Mutex::new(String::new()),
            problem: Mutex::new(None),
            requests: Mutex::new(VecDeque::new()),
            sink,
        }
    }

    /// A log line with the time, to the window's Log tab.
    pub fn log(&self, msg: &str) {
        (self.sink)(format!("{} {msg}", chrono::Local::now().format("%H:%M:%S")));
    }

    pub fn sender(&self) -> Sink {
        self.sink.clone()
    }

    pub fn request(&self, r: Request) {
        if let Ok(mut q) = self.requests.lock() {
            q.push_back(r);
        }
    }

    fn take_requests(&self) -> Vec<Request> {
        self.requests.lock().map(|mut q| q.drain(..).collect()).unwrap_or_default()
    }

    pub fn set_state(&self, s: &str) {
        if let Ok(mut st) = self.state.lock() {
            *st = s.to_string();
        }
    }

    pub fn state(&self) -> String {
        self.state.lock().map(|s| s.clone()).unwrap_or_default()
    }

    pub fn set_heard(&self, text: &str) {
        if let Ok(mut h) = self.heard.lock() {
            *h = text.to_string();
        }
    }

    pub fn heard(&self) -> String {
        self.heard.lock().map(|s| s.clone()).unwrap_or_default()
    }

    pub fn set_problem(&self, p: &str) {
        if let Ok(mut x) = self.problem.lock() {
            *x = Some(p.to_string());
        }
    }

    pub fn take_problem(&self) -> Option<String> {
        self.problem.lock().ok().and_then(|mut p| p.take())
    }

    pub fn stopped(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }

    pub fn paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    pub fn pid(&self) -> u32 {
        self.pid.load(Ordering::Relaxed)
    }
}
