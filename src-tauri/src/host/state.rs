//! What the window process keeps while it runs: the folders, the configuration, the listener's
//! bridge, the last reading of the Claude console, the log/note feed for the window and the
//! updater's state.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use achaja_core::config::Config;
use achaja_core::console::screen::Screen;
use achaja_core::listener::Bridge;
use achaja_core::paths::Paths;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct FeedItem {
    pub seq: u64,
    /// "log" (Log tab) or "note" (a line in the conversation; "@key" is a text key of the window).
    pub kind: String,
    pub text: String,
}

#[derive(Default)]
pub struct Feed {
    items: Vec<FeedItem>,
    next: u64,
}

impl Feed {
    pub fn push(&mut self, kind: &str, text: String) {
        self.next += 1;
        self.items.push(FeedItem { seq: self.next, kind: kind.into(), text });
        if self.items.len() > 3000 {
            self.items.drain(..1000);
        }
    }

    pub fn since(&self, seq: u64) -> Vec<FeedItem> {
        self.items.iter().filter(|i| i.seq > seq).cloned().collect()
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateView {
    /// "", "checking", "downloading", "ready", "latest", "error"
    pub phase: String,
    pub version: String,
    pub pct: u32,
    pub error: String,
    pub manual: bool,
}

pub struct AppState {
    pub paths: Paths,
    pub cfg: RwLock<Config>,
    pub bridge: RwLock<Option<Arc<Bridge>>>,
    pub listener: Mutex<Option<std::thread::JoinHandle<()>>>,
    pub feed: Mutex<Feed>,
    pub screen: Mutex<Option<Screen>>,
    pub term_open: AtomicBool,
    pub visible: AtomicBool,
    pub quitting: AtomicBool,
    pub last_start: Mutex<f64>,
    pub update: Mutex<UpdateView>,
    /// Started with --settings or asked for them from the tray: the window opens on Settings
    /// (taken once by the page).
    pub open_settings: AtomicBool,
    /// No config.json yet: the first-run settings wizard comes first, the listener after it.
    pub first_run: AtomicBool,
    /// The app runs without the listener (ACHAJA_NO_LISTENER, for tests of the window).
    pub no_listener: bool,
}

impl AppState {
    pub fn new(paths: Paths, cfg: Config, open_settings: bool) -> Self {
        AppState {
            paths,
            cfg: RwLock::new(cfg),
            bridge: RwLock::new(None),
            listener: Mutex::new(None),
            feed: Mutex::new(Feed::default()),
            screen: Mutex::new(None),
            term_open: AtomicBool::new(false),
            visible: AtomicBool::new(true),
            quitting: AtomicBool::new(false),
            last_start: Mutex::new(0.0),
            update: Mutex::new(UpdateView::default()),
            open_settings: AtomicBool::new(open_settings),
            first_run: AtomicBool::new(false),
            no_listener: std::env::var_os("ACHAJA_NO_LISTENER").is_some(),
        }
    }

    pub fn cfg(&self) -> Config {
        self.cfg.read().map(|c| c.clone()).unwrap_or_else(|_| Config::new(serde_json::Value::Null))
    }

    pub fn bridge(&self) -> Option<Arc<Bridge>> {
        self.bridge.read().ok().and_then(|b| b.clone())
    }

    /// Claude's PID when the session is alive.
    pub fn claude_pid(&self) -> Option<u32> {
        self.bridge().map(|b| b.pid()).filter(|p| *p > 0 && achaja_core::process::pid_alive(*p))
    }

    pub fn log(&self, text: &str) {
        let line = format!("{} {text}", chrono::Local::now().format("%H:%M:%S"));
        if let Ok(mut f) = self.feed.lock() {
            f.push("log", line);
        }
    }

    pub fn note(&self, text: &str) {
        if let Ok(mut f) = self.feed.lock() {
            f.push("note", text.to_string());
        }
    }

    pub fn quitting(&self) -> bool {
        self.quitting.load(Ordering::Relaxed)
    }

    pub fn lang(&self) -> &'static str {
        self.cfg().lang2()
    }
}
