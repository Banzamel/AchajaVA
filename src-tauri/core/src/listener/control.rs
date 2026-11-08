//! Driving Claude Code's dictation. Space toggles recording, so the screen is read first: when
//! Claude already records, Space would stop it; in a choice dialog it could pick an option.

use std::time::{Duration, Instant};

use super::Bridge;
use crate::config::Config;
use crate::console::screen::{markers, parse, Markers, Screen};
use crate::json::now;
use crate::paths::Paths;
use crate::state::{self, RECORDING};
use crate::voice::playback::beep;

#[derive(PartialEq, Clone, Copy)]
pub enum Mode {
    Idle,
    Recording,
}

pub struct Ctl<'a> {
    pub paths: &'a Paths,
    pub cfg: Config,
    pub bridge: &'a Bridge,
    pub pid: u32,
    pub markers: Markers,
    pub screen_sync: bool,
    pub mode: Mode,
    pub started: f64,
    pub last_speech: f64,
    pub last_inject: Instant,
    pub screen_ok: bool,
    pub reset_free: bool,
}

fn sleep(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

impl<'a> Ctl<'a> {
    pub fn new(paths: &'a Paths, cfg: Config, bridge: &'a Bridge, pid: u32) -> Self {
        let markers = markers(&cfg);
        let screen_sync = cfg.bool_or("wake.screen_sync", true);
        Ctl { paths, cfg, bridge, pid, markers, screen_sync, mode: Mode::Idle, started: 0.0, last_speech: 0.0, last_inject: Instant::now(), screen_ok: false, reset_free: false }
    }

    pub fn beep(&self, freq: u32, ms: u32) {
        if self.cfg.bool_or("wake.beep", true) {
            beep(freq, ms);
        }
    }

    pub fn press(&mut self, keys: &[&str]) -> bool {
        self.last_inject = Instant::now();
        let keys: Vec<String> = keys.iter().map(|k| k.to_string()).collect();
        match crate::console::write_keys(self.pid, &keys) {
            Ok(()) => true,
            Err(e) => {
                self.bridge.log(&format!("BLAD wstrzykiwania klawiszy: {e}"));
                false
            }
        }
    }

    pub fn screen(&self) -> Option<Screen> {
        if !self.screen_sync {
            return None;
        }
        crate::console::read_screen(self.pid, 0).ok().map(|lines| parse(&lines, &self.markers))
    }

    /// Claude records (from the "● REC" bar); None when the screen cannot be read.
    pub fn recording_now(&mut self) -> Option<bool> {
        let rec = self.screen().map(|s| s.rec);
        self.screen_ok = rec.is_some();
        rec
    }

    /// After Space Claude needs a moment to show REC; an unreadable screen counts as success.
    fn wait_for_recording(&mut self) -> bool {
        let until = Instant::now() + Duration::from_millis(2500);
        while Instant::now() < until {
            sleep(250);
            match self.screen().map(|s| s.rec) {
                None | Some(true) => return true,
                _ => {}
            }
        }
        false
    }

    fn enter_recording(&mut self, reason: &str) {
        self.mode = Mode::Recording;
        self.started = now();
        self.last_speech = now();
        state::stamp(self.paths, RECORDING); // other processes (mail) do not speak over it
        self.bridge.log(&format!("{reason} Stan: NAGRYWAM"));
    }

    pub fn start_recording(&mut self, reason: &str) {
        self.reset_free = true;
        self.bridge.set_heard("");
        let screen = self.screen();
        if screen.as_ref().is_some_and(|s| s.rec) {
            self.beep(880, 120);
            return self.enter_recording(&format!("{reason} Claude juz nagrywa - nie naciskam spacji."));
        }
        if screen.as_ref().is_some_and(|s| s.dialog) {
            self.beep(330, 300);
            self.bridge.set_problem("dialog");
            return self.bridge.log(&format!("{reason} Claude czeka na decyzje w oknie wyboru - nie nagrywam (zakladka Terminal)."));
        }
        self.beep(880, 120);
        // ctrl+u clears the box: tap starts recording only in an empty box
        if self.press(&["ctrl+u", "space"]) {
            if !self.wait_for_recording() {
                self.beep(330, 300);
                self.bridge.set_problem("rec_failed");
                let lines: Vec<String> = self.screen().map(|s| s.lines.into_iter().filter(|l| !l.trim().is_empty()).collect()).unwrap_or_default();
                let tail = lines[lines.len().saturating_sub(4)..].join(" | ");
                return self.bridge.log(&format!("{reason} Claude nie wlaczyl nagrywania. Na ekranie: {}", crate::text::truncate_chars(&tail, 300)));
            }
            self.enter_recording(reason);
            super::replies::assign_claude_microphone(self.paths, self.bridge, self.pid, &self.cfg.str_or("wake.microphone", ""));
        }
    }

    pub fn to_idle(&mut self, msg: &str) {
        self.mode = Mode::Idle;
        self.bridge.set_heard("");
        state::clear(self.paths, RECORDING);
        self.bridge.log(&format!("{msg} Stan: czekam na 'Achaja'"));
    }

    /// Recording: Space sends; Claude already stopped: the text waits in the box, Enter sends it.
    pub fn send(&mut self) {
        let key = if self.recording_now() == Some(false) { "enter" } else { "space" };
        self.press(&[key]);
        self.beep(1175, 120);
        self.to_idle("Wyslano prompt.");
    }

    pub fn cancel(&mut self) {
        if self.recording_now() != Some(false) {
            self.press(&["esc"]); // Esc outside recording would interrupt Achaja's work
            sleep(400);
        }
        self.press(&["ctrl+u"]);
        self.beep(440, 300);
        self.to_idle("Anulowano.");
    }

    /// Types and sends. Claude Code sometimes skips the first Enter after typed text, so the
    /// screen is checked and Enter pressed again when needed.
    pub fn submit(&mut self, text: &str) {
        let head: String = text.chars().take(20).collect();
        self.press(&["ctrl+u", &format!("text:{text}")]);
        let until = Instant::now() + Duration::from_secs(2);
        while Instant::now() < until {
            sleep(200);
            if self.screen().map_or(true, |s| s.input.contains(&head)) {
                break;
            }
        }
        sleep(500);
        self.press(&["enter"]);
        for _ in 0..2 {
            sleep(1000);
            match self.screen() {
                Some(s) if !s.input.is_empty() && s.input.contains(&head) => {
                    self.bridge.log("Tekst nadal czeka w polu Claude - ponawiam Enter.");
                    self.press(&["enter"]);
                }
                _ => return,
            }
        }
    }

    pub fn new_conversation(&mut self) {
        if self.recording_now() != Some(false) {
            self.press(&["esc"]);
            sleep(400);
        }
        self.submit("/clear");
        self.beep(660, 120);
        self.beep(880, 120);
        self.to_idle("Nowa rozmowa (/clear).");
    }
}
