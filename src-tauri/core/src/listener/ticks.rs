//! What the listener checks between audio blocks: Claude's REC bar (it may start or stop
//! recording on its own), silence and length limits, conversation mode after a question,
//! "still working" reminders, the plan (reminders, timed commands, background jobs) and the
//! daily backup.

use rand::seq::IndexedRandom;

use super::control::{Ctl, Mode};
use crate::json::now;
use crate::plan::{jobs, store};
use crate::state::{self, FOLLOWUP, SPEECH_DONE};
use crate::voice::speak::say;

pub struct Timers {
    pub last_sync: f64,
    pub last_reminder: f64,
    pub last_plan: f64,
    pub last_backup: f64,
    pub last_limits: f64,
}

impl Timers {
    pub fn new() -> Self {
        // the first backup check about a minute after the start
        Timers { last_sync: 0.0, last_reminder: 0.0, last_plan: 0.0, last_backup: now() - 540.0, last_limits: now() - 240.0 }
    }
}

/// Achaja asked a question (Stop hook) and has just finished saying it.
fn followup_ready(ctl: &Ctl) -> bool {
    let asked = state::at(ctl.paths, FOLLOWUP);
    if asked == 0.0 {
        return false;
    }
    if now() - asked > 300.0 {
        state::clear(ctl.paths, FOLLOWUP);
        return false;
    }
    if state::at(ctl.paths, SPEECH_DONE) > asked && !state::speaking(ctl.paths) {
        state::clear(ctl.paths, FOLLOWUP);
        return true;
    }
    false
}

pub fn tick(ctl: &mut Ctl, t: &mut Timers) {
    let n = now();
    let since_inject = ctl.last_inject.elapsed().as_secs_f64();
    let every = if ctl.mode == Mode::Recording { 1.0 } else { 2.0 };
    if ctl.screen_sync && n - t.last_sync >= every && since_inject > 2.5 {
        t.last_sync = n;
        let rec = ctl.recording_now();
        if ctl.mode == Mode::Recording && rec == Some(false) {
            ctl.to_idle("Claude zakonczyl nagrywanie (brak REC na ekranie).");
        } else if ctl.mode == Mode::Idle && rec == Some(true) {
            ctl.reset_free = true;
            ctl.mode = Mode::Recording;
            ctl.started = n;
            ctl.last_speech = n;
            state::stamp(ctl.paths, state::RECORDING);
            ctl.bridge.log("Claude nagrywa (wlaczone poza mna) - dolaczam. Stan: NAGRYWAM");
        }
    }
    let max_rec = ctl.cfg.f64_or("wake.max_recording_seconds", 110.0);
    let silence = ctl.cfg.f64_or("wake.silence_stop_seconds", 17.0);
    if ctl.mode == Mode::Recording && !ctl.screen_ok && (n - ctl.started > max_rec || n - ctl.started.max(ctl.last_speech) > silence) {
        ctl.to_idle("Cisza lub limit czasu - Claude sam zakonczyl nagrywanie.");
    }
    if ctl.mode == Mode::Idle && ctl.cfg.bool_or("wake.auto_listen_after_question", true) && !ctl.bridge.paused() && followup_ready(ctl) {
        ctl.start_recording("Achaja zadala pytanie - slucham odpowiedzi.");
    }
    let remind_after = ctl.cfg.f64_or("voice.working_reminder_after_seconds", 30.0);
    if ctl.mode == Mode::Idle && remind_after > 0.0 {
        let (work, since, _) = state::work(ctl.paths);
        let busy_for = n - since;
        let every = ctl.cfg.f64_or("voice.working_reminder_every_seconds", 60.0);
        let max = ctl.cfg.f64_or("voice.working_reminder_max_seconds", 900.0);
        if work == "busy" && (remind_after..max).contains(&busy_for) && (t.last_reminder < since || n - t.last_reminder >= every) && !state::speaking(ctl.paths) {
            t.last_reminder = n;
            let phrases = ctl.cfg.strs("voice.working_phrases");
            let phrase = phrases.choose(&mut rand::rng()).cloned().unwrap_or_else(|| "Nadal pracuję.".into());
            say(ctl.paths, &phrase);
            ctl.bridge.log(&format!("Achaja pracuje od {} s - przypomnienie glosowe.", busy_for as u64));
        }
    }
    // the plan's usage every 5 minutes, in its own thread (a web request); not into a recording
    if n - t.last_limits >= 300.0 && ctl.mode == Mode::Idle {
        t.last_limits = n;
        let paths = ctl.paths.clone();
        std::thread::spawn(move || crate::tokens::guard::check_plan(&paths));
    }
    if n - t.last_plan >= 15.0 {
        t.last_plan = n;
        plan(ctl);
        if n - t.last_backup >= 600.0 {
            t.last_backup = n;
            if crate::backup::due(ctl.paths) {
                let _ = crate::process::spawn_self(&["backup", "auto"], &ctl.paths.data);
                ctl.bridge.log("Kopia zapasowa ustawien i danych (achaja backup auto).");
            }
        }
    }
}

fn plan(ctl: &mut Ctl) {
    let mut kinds = vec!["job"]; // background jobs always: they are a separate Claude session
    if ctl.mode == Mode::Idle {
        kinds.push("say"); // not into a recording
        if state::work(ctl.paths).0 == "idle" && !state::speaking(ctl.paths) {
            kinds.push("prompt"); // a command only when Achaja does nothing
        }
    }
    for due in store::take_due(ctl.paths, &kinds, None) {
        match due.kind.as_str() {
            "say" => {
                say(ctl.paths, &due.text);
                ctl.bridge.log(&format!("Przypomnienie: {}", due.text));
            }
            "job" => {
                // a recurring job waits while the plan's limit or the daily cost is used up
                if let Some(why) = due.item.repeat.as_ref().and_then(|_| crate::tokens::guard::jobs_paused_now(ctl.paths)) {
                    jobs::skipped(ctl.paths, &due.item, &why);
                    ctl.bridge.log(&format!("Zadanie w tle '{}' pominiete: {why}", due.item.name.clone().unwrap_or_default()));
                    continue;
                }
                let started = jobs::start(ctl.paths, &due.item);
                ctl.bridge.log(&format!("Zadanie w tle '{}': {started}", due.item.name.clone().unwrap_or_default()));
            }
            _ => {
                ctl.submit(&format!("{} {}", crate::plan::SCHEDULED, due.text));
                ctl.bridge.log(&format!("Zaplanowane polecenie: {}", due.text));
            }
        }
    }
}
