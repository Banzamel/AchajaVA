//! `achaja agent run <project>`: Claude in the project's folder (with the project's own
//! .claude, CLAUDE.md and skills), its progress in state\agents\<project>.log (the Agents tab),
//! and the report printed at the end for Achaja to summarize. Successive orders to the same
//! project continue its conversation (--resume) unless --new. An agent without any progress for
//! agents.hang_minutes is stopped.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};

use super::stream::{parse_line, report, Event};
use super::{protected_settings, read_map, write_map, AGENT_PROMPT};
use crate::config::Config;
use crate::error::{AppError, Result};
use crate::json::{now, write_text};
use crate::paths::Paths;
use crate::process::{claude_command, find_claude, kill_tree, pid_alive};

struct Log(std::fs::File);

impl Log {
    fn line(&mut self, text: &str) {
        let _ = writeln!(self.0, "{text}");
        let _ = self.0.flush();
    }
}

pub fn run(paths: &Paths, cfg: &Config, name: &str, folder: &PathBuf, prompt: &str, new: bool) -> Result<i32> {
    let dir = paths.agents();
    let mut running = read_map(&dir.join("running.json"));
    if let Some(pid) = running.get(name).and_then(|r| r["pid"].as_u64()).filter(|p| pid_alive(*p as u32)) {
        return Err(AppError::msg(format!("Agent projektu {name} juz pracuje (PID {pid}). Poczekaj na wynik albo uzyj: achaja.cmd agent stop {name}")));
    }
    let claude = find_claude().ok_or_else(|| AppError::msg("Nie znaleziono 'claude' w PATH."))?;
    let mut sessions = read_map(&dir.join("sessions.json"));
    let mut cmd = claude_command(&claude);
    cmd.args(["-p", "--output-format", "stream-json", "--verbose", "--permission-mode", &cfg.str_or("agents.permission_mode", "auto")]);
    cmd.args(["--append-system-prompt", &AGENT_PROMPT.replace("{name}", name), "--settings", &protected_settings(paths)]);
    let resumed = !new && sessions.get(name).and_then(Value::as_str).is_some();
    if let Some(sid) = sessions.get(name).and_then(Value::as_str).filter(|_| resumed) {
        cmd.args(["--resume", sid]);
    }
    let log_path = dir.join(format!("{name}.log"));
    let mut log = Log(std::fs::File::create(&log_path)?);
    log.line(&format!("=== Agent projektu {name} ({}) ===", folder.display()));
    let mode = if resumed { "kontynuacja rozmowy" } else { "nowa rozmowa" };
    log.line(&format!("Start: {}  |  {mode}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")));
    log.line(&format!("Zlecenie: {prompt}\n"));
    cmd.current_dir(folder).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn()?;
    let pid = child.id();
    running.insert(name.to_string(), json!({"pid": pid, "started": now(), "prompt": crate::text::truncate_chars(prompt, 300)}));
    write_map(&dir.join("running.json"), &running);
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(prompt.as_bytes())?;
    }
    // a hung agent (a tool that never returns) ends itself after hang_minutes without progress
    let hang = cfg.f64_or("agents.hang_minutes", 45.0) * 60.0;
    let last = Arc::new(AtomicU64::new(now() as u64));
    let hung = Arc::new(AtomicBool::new(false));
    let (l2, h2) = (last.clone(), hung.clone());
    std::thread::spawn(move || {
        while pid_alive(pid) {
            std::thread::sleep(Duration::from_secs(15));
            if hang > 0.0 && pid_alive(pid) && now() - l2.load(Ordering::Relaxed) as f64 > hang {
                h2.store(true, Ordering::Relaxed);
                kill_tree(pid);
                return;
            }
        }
    });
    let mut result: Option<Value> = None;
    let stderr = child.stderr.take();
    if let Some(out) = child.stdout.take() {
        for line in BufReader::new(out).lines().map_while(std::result::Result::ok) {
            last.store(now() as u64, Ordering::Relaxed);
            for ev in parse_line(&line) {
                match ev {
                    Event::Session(sid) => {
                        sessions.insert(name.to_string(), json!(sid));
                        write_map(&dir.join("sessions.json"), &sessions);
                    }
                    Event::Log(text) => log.line(&text),
                    Event::Result(r) => result = Some(r),
                    Event::Nothing => {}
                }
            }
        }
    }
    let status = child.wait()?;
    if let Some(err) = stderr {
        for line in BufReader::new(err).lines().map_while(std::result::Result::ok).filter(|l| !l.trim().is_empty()) {
            log.line(&format!("  {line}"));
        }
    }
    let mut running = read_map(&dir.join("running.json"));
    running.remove(name);
    write_map(&dir.join("running.json"), &running);
    let Some(result) = result else {
        let text = if hung.load(Ordering::Relaxed) {
            let t = format!(
                "Agent projektu {name} zawiesil sie - przez {} min nie bylo postepu, wiec go przerwalam. Ostatnie kroki sa w logu: {}",
                (hang / 60.0).round(),
                log_path.display()
            );
            let _ = write_text(&dir.join(format!("{name}-last.md")), &format!("# Wynik agenta projektu {name}\n\n{t}\n"));
            t
        } else {
            format!("Agent projektu {name} zakonczyl sie bez wyniku (kod {}). Log: {}", status.code().unwrap_or(-1), log_path.display())
        };
        log.line(&text);
        log.line("=== KONIEC ===");
        println!("{text}");
        return Ok(1);
    };
    if let Some(sid) = result["session_id"].as_str() {
        sessions.insert(name.to_string(), json!(sid));
        write_map(&dir.join("sessions.json"), &sessions);
    }
    let text = report(name, &result);
    write_text(&dir.join(format!("{name}-last.md")), &text)?;
    log.line(&format!("\n{text}"));
    log.line("=== KONIEC ===");
    println!("{text}");
    Ok(0)
}
