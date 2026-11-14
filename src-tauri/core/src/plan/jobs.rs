//! Background jobs from the plan (plan add --job): a separate headless Claude session in the
//! data folder does them on its own, with Achaja's rules, mail and Home Assistant but without a
//! conversation. The gate lets it act only in the categories it was allowed; the report lands in
//! the chat and the Plan tab, and its [GŁOS] part is spoken when nobody records ("CISZA" = nothing
//! worth saying). A run is in job_claude.rs; one that failed for a passing reason (an
//! overloaded API) is tried once more after jobs.retry_minutes. Every run's tokens go to the
//! ledger; a recurring job skipped because of the limits is noted in the history.

use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::store::Item;
use super::job_claude::{self, Outcome};
use crate::chat;
use crate::config;
use crate::json::{now, read_json, read_text, remove, write_json, write_text};
use crate::paths::Paths;
use crate::process::{pid_alive, spawn_self};
use crate::state::RECORDING;
use crate::tokens::ledger::{self, Entry};

const SILENT: &str = "CISZA";

fn dir(paths: &Paths) -> std::path::PathBuf {
    paths.state().join("jobs")
}

/// Starts the job in its own process (the listener never waits); a description or why not.
pub fn start(paths: &Paths, item: &Item) -> String {
    let d = dir(paths);
    let pid_file = d.join(format!("{}.pid", item.id));
    if read_text(&pid_file).and_then(|t| t.trim().parse().ok()).is_some_and(pid_alive) {
        return "juz dziala".into();
    }
    let spec = d.join(format!("{}-spec.json", item.id));
    if write_json(&spec, item).is_err() {
        return "nie udalo sie zapisac zlecenia".into();
    }
    match spawn_self(&["jobs", "exec", &spec.to_string_lossy()], &paths.data) {
        Ok(pid) => {
            let _ = write_text(&pid_file, &pid.to_string());
            format!("PID {pid}")
        }
        Err(e) => format!("blad: {e}"),
    }
}

fn speak_when_free(paths: &Paths, text: &str) {
    let deadline = Instant::now() + Duration::from_secs(600);
    while paths.state_file(RECORDING).exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_secs(3));
    }
    crate::voice::speak::say(paths, text);
}

/// `achaja jobs exec <spec>`: the job's process.
pub fn execute(paths: &Paths, spec: &std::path::Path) -> i32 {
    let Ok(item) = serde_json::from_value::<Item>(read_json(spec, Value::Null)) else { return 1 };
    let name = item.name.clone().unwrap_or_else(|| crate::text::truncate_chars(&item.text, 40));
    let (result, ok) = run_logged(paths, &item, &name);
    let stamp = chrono::Local::now().format("%Y-%m-%d %H:%M");
    let report = format!("# Zadanie w tle: {name}\n{stamp} | {}\n\n{result}\n", if ok { "OK" } else { "BŁĄD" });
    let _ = write_text(&dir(paths).join(format!("{}-last.md", item.id)), &report);
    let voice = crate::hooks::stop::voice_text(&result);
    let voice = if crate::hooks::stop::has_marker(&result) { voice } else { String::new() };
    add_history(paths, json!({"id": item.id, "name": name, "at": now(), "ok": ok, "voice": crate::text::truncate_chars(&voice, 300)}));
    chat::append(paths, "assistant", &format!("Zadanie w tle „{name}” ({stamp}):\n\n{result}"));
    let spoken = if !ok && voice.is_empty() { format!("Zadanie {name} się nie udało. Szczegóły są w zakładce Plan.") } else { voice };
    if !spoken.is_empty() && spoken.trim_matches([' ', '.']).to_uppercase() != SILENT {
        speak_when_free(paths, &spoken);
    }
    remove(&dir(paths).join(format!("{}.pid", item.id)));
    0
}

/// Runs the job (and once more after a passing failure), each run into the token ledger.
fn run_logged(paths: &Paths, item: &Item, name: &str) -> (String, bool) {
    let effort = config::load_or_default(paths).str_or("achaja.effort", "");
    let wait = config::load_or_default(paths).f64_or("jobs.retry_minutes", 5.0);
    let mut retries = 0;
    loop {
        let Outcome { text, ok, transient, result } = job_claude::run(paths, item);
        if !result.is_null() {
            let mut entry = Entry::from_result("job", name, &result, &effort);
            entry.retries = retries;
            ledger::append(paths, &entry);
        }
        if ok || !transient || retries > 0 || wait <= 0.0 {
            return (text, ok);
        }
        retries += 1;
        std::thread::sleep(Duration::from_secs_f64(wait * 60.0));
    }
}

fn add_history(paths: &Paths, entry: Value) {
    let mut history: Vec<Value> = read_json(&dir(paths).join("history.json"), Vec::new());
    history.insert(0, entry);
    history.truncate(100);
    let _ = write_json(&dir(paths).join("history.json"), &history);
}

/// A recurring job not started because of the limits: a line in the history (the Plan tab).
pub fn skipped(paths: &Paths, item: &Item, why: &str) {
    let name = item.name.clone().unwrap_or_else(|| crate::text::truncate_chars(&item.text, 40));
    add_history(paths, json!({"id": item.id, "name": name, "at": now(), "ok": false, "voice": format!("Pominięte: {why}")}));
}

pub fn history(paths: &Paths) -> Vec<Value> {
    read_json(&dir(paths).join("history.json"), Vec::new())
}

pub fn last_report(paths: &Paths, id: u64) -> Option<String> {
    read_text(&dir(paths).join(format!("{id}-last.md")))
}

pub fn last_report_mtime(paths: &Paths, id: u64) -> f64 {
    crate::json::mtime(&dir(paths).join(format!("{id}-last.md")))
}
