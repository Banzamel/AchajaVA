//! Project agents: Claude started in a project's folder with that project's own rules, its
//! progress in the Agents tab and its report back to Achaja (`achaja agent ...`).

pub mod resolve;
pub mod run;
pub mod stream;

use std::path::Path;

use clap::Subcommand;
use serde_json::{json, Map, Value};

use crate::config;
use crate::error::{AppError, Result};
use crate::json::{mtime, now, read_json, write_json};
use crate::paths::Paths;
use crate::process::{kill_tree, pid_alive};

pub const AGENT_PROMPT: &str = "Pracujesz jako agent projektu \"{name}\" na zlecenie Achai - glosowego koordynatora uzytkownika. Uzytkownik jest z dala od komputera i NIE odpowie na pytania w trakcie Twojej pracy.\n- Wykonaj zadanie samodzielnie, zgodnie z zasadami tego projektu (jego CLAUDE.md, skille, ustawienia).\n- Akcji ryzykownych (git push, deploy, produkcja, usuwanie danych, migracje na bazie, wydatki) NIE wykonuj - opisz je jako propozycje w podsumowaniu.\n- Gdy brakuje decyzji, wybierz rozsadne domyslne rozwiazanie, jesli jest odwracalne; w przeciwnym razie zapytaj w podsumowaniu.\n- Zakoncz odpowiedz sekcja \"PODSUMOWANIE\" po polsku: co zrobiono, zmienione pliki, wyniki testow/buildow, problemy, oraz \"PYTANIA\" do uzytkownika (jesli sa).";

pub fn read_map(path: &Path) -> Map<String, Value> {
    read_json::<Value>(path, Value::Null).as_object().cloned().unwrap_or_default()
}

pub fn write_map(path: &Path, map: &Map<String, Value>) {
    let _ = write_json(path, map);
}

/// A path in a Claude Code permission rule: C:\x\y -> //c/x/y.
pub fn rule_path(path: &Path) -> String {
    let p = path.to_string_lossy().replace('\\', "/");
    let bytes = p.as_bytes();
    let p = if bytes.len() > 2 && bytes[1] == b':' && bytes[2] == b'/' { format!("/{}{}", p[..1].to_lowercase(), &p[2..]) } else { p };
    if p.starts_with('/') {
        format!("/{p}")
    } else {
        p
    }
}

/// Agents work in other folders but neither read nor change Achaja's files: tokens and logs,
/// the hooks settings and the configuration (a way around the gate or to take the token).
pub fn protected_settings(paths: &Paths) -> String {
    let d = &paths.data;
    let targets = [d.join("state").join("**"), d.join("config.json"), d.join(".mcp.json"), d.join(".claude").join("settings.json"), d.join(".claude").join("settings.local.json")];
    let deny: Vec<String> = targets.iter().flat_map(|t| {
        let r = rule_path(t);
        [format!("Read({r})"), format!("Edit({r})"), format!("Write({r})")]
    }).collect();
    json!({"permissions": {"deny": deny}}).to_string()
}

#[derive(Debug, Subcommand)]
pub enum AgentCmd {
    /// Projekty i aliasy
    List,
    /// Kto pracuje, ostatnie wyniki
    Status,
    /// Zleca zadanie (czeka na wynik)
    Run {
        project: String,
        #[arg(long)]
        prompt: Option<String>,
        #[arg(long)]
        prompt_file: Option<String>,
        #[arg(long)]
        new: bool,
    },
    /// Pelny ostatni wynik agenta
    Last { project: String },
    /// Przerywa prace agenta
    Stop { project: String },
}

/// Agents now working: name -> (pid, started, prompt).
pub fn running(paths: &Paths) -> Map<String, Value> {
    let mut map = read_map(&paths.agents().join("running.json"));
    map.retain(|_, r| r["pid"].as_u64().is_some_and(|p| pid_alive(p as u32)));
    map
}

pub fn stop(paths: &Paths, name: &str) -> bool {
    match running(paths).get(name).and_then(|r| r["pid"].as_u64()) {
        Some(pid) => {
            kill_tree(pid as u32);
            true
        }
        None => false,
    }
}

pub fn cli(paths: &Paths, cmd: AgentCmd) -> Result<i32> {
    let cfg = config::load(paths)?;
    match cmd {
        AgentCmd::List => {
            let projects = resolve::all_projects(&cfg);
            if projects.is_empty() {
                let roots = cfg.strs("agents.project_roots").join(", ");
                println!("Brak projektow. Ustaw agents.project_roots w config.json (teraz: {}).", if roots.is_empty() { "(brak)" } else { &roots });
            }
            let sessions = read_map(&paths.agents().join("sessions.json"));
            let aliases = resolve::aliases(&cfg);
            for (name, path) in projects {
                let al: Vec<&String> = aliases.iter().filter(|(_, t)| **t == name).map(|(a, _)| a).collect();
                let al = if al.is_empty() { "-".to_string() } else { al.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ") };
                let has = path.join(".claude").is_dir() || path.join("CLAUDE.md").exists();
                println!("{name:25} aliasy: {al:25} .claude: {:4} rozmowa: {}", if has { "tak" } else { "nie" }, if sessions.contains_key(&name) { "tak" } else { "nie" });
            }
        }
        AgentCmd::Status => {
            let alive = running(paths);
            if alive.is_empty() {
                println!("Zaden agent teraz nie pracuje.");
            }
            for (n, r) in &alive {
                let mins = ((now() - r["started"].as_f64().unwrap_or(now())) / 60.0).round();
                println!("PRACUJE: {n} od {mins} min - {}", crate::text::truncate_chars(r["prompt"].as_str().unwrap_or(""), 120));
            }
            let mut lasts: Vec<_> = std::fs::read_dir(paths.agents()).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.to_string_lossy().ends_with("-last.md")).collect();
            lasts.sort_by(|a, b| mtime(b).total_cmp(&mtime(a)));
            for f in lasts.iter().take(5) {
                let name = f.file_name().map(|n| n.to_string_lossy().trim_end_matches("-last.md").to_string()).unwrap_or_default();
                let when = crate::plan::schedule::from_ts(mtime(f)).format("%H:%M %d.%m");
                println!("Ostatni wynik: {name} ({when})");
            }
        }
        AgentCmd::Run { project, prompt, prompt_file, new } => {
            let (name, folder) = resolve::resolve(&project, &cfg)?;
            let text = match (prompt_file, prompt) {
                (Some(f), _) => std::fs::read_to_string(f)?,
                (None, Some(p)) => p,
                _ => String::new(),
            };
            if text.trim().is_empty() {
                return Err(AppError::msg("Brak tresci zlecenia (--prompt lub --prompt-file)."));
            }
            return run::run(paths, &cfg, &name, &folder, &text, new);
        }
        AgentCmd::Last { project } => {
            let (name, _) = resolve::resolve(&project, &cfg)?;
            let file = paths.agents().join(format!("{name}-last.md"));
            println!("{}", crate::json::read_text(&file).unwrap_or_else(|| format!("Brak wynikow dla {name}.")));
        }
        AgentCmd::Stop { project } => {
            let (name, _) = resolve::resolve(&project, &cfg)?;
            println!("{}", if stop(paths, &name) { format!("Przerwano agenta {name}.") } else { format!("Agent {name} nie pracuje.") });
        }
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_paths_are_posix() {
        assert_eq!(rule_path(Path::new(r"C:\Banzamel\Achaja\data\state\**")), "//c/Banzamel/Achaja/data/state/**");
    }
}
