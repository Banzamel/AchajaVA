//! `achaja claude <command>`: the account, plan usage, sessions and tidying, by voice.

use clap::Subcommand;

use super::account::{account, usage};
use super::cleanup::{clean, delete_project, delete_session, storage};
use super::labels::{find_project, reset_text, window_label};
use super::sessions::{human, projects, projects_dir, sessions, size_of};
use crate::error::Result;
use crate::paths::Paths;

#[derive(Debug, Subcommand)]
pub enum ClaudeCmd {
    /// Zalogowane konto (e-mail, organizacja, plan)
    Account,
    /// Wykorzystanie limitow planu
    Usage,
    /// Konto + limity + miejsce, krotko (do powiedzenia)
    Summary,
    /// Zuzycie tokenow i szacowany koszt (dzis, 7 dni); --details: tabela per rodzaj, model i dzien
    Stats {
        #[arg(long)]
        details: bool,
    },
    /// Projekty z sesjami Claude
    Projects,
    /// Sesje projektu
    Sessions { project: String },
    /// Ile zajmuja foldery ~/.claude
    Storage,
    /// Usuwa zapis sesji
    DeleteSession { project: String, session: String },
    /// Usuwa wszystkie sesje projektu (pliki projektu zostaja)
    DeleteProject { project: String },
    /// Stare sesje i pliki pomocnicze; bez --apply tylko pokazuje
    Clean {
        #[arg(long, default_value_t = 30)]
        days: u64,
        #[arg(long)]
        apply: bool,
    },
}

fn date(ts: f64) -> String {
    chrono::DateTime::from_timestamp(ts as i64, 0)
        .map(|d| d.with_timezone(&chrono::Local).format("%d.%m.%Y %H:%M").to_string())
        .unwrap_or_default()
}

/// A few sentences to speak: account, usage, space.
pub fn summary(paths: &Paths) -> String {
    let a = account();
    let mut parts = Vec::new();
    if !a.email.is_empty() {
        let plan = if a.plan.is_empty() { String::new() } else { format!(", plan {}", a.plan) };
        parts.push(format!("Konto {}{plan}.", a.email));
    }
    match crate::runtime::block_on(usage(paths, false)) {
        Ok(windows) => {
            for w in windows {
                let reset = reset_text(w.resets_at.as_deref(), "pl", chrono::Local::now());
                let reset = if reset.is_empty() { String::new() } else { format!(", {reset}") };
                parts.push(format!("Limit {}: wykorzystane {} procent{reset}.", window_label(&w.key, "pl"), w.percent.round()));
            }
        }
        Err(e) => parts.push(e.to_string()),
    }
    parts.push(format!("Sesje Claude zajmują {} w {} projektach.", human(size_of(&projects_dir())), projects().len()));
    parts.join(" ")
}

pub fn run(paths: &Paths, cmd: ClaudeCmd) -> Result<i32> {
    match cmd {
        ClaudeCmd::Account => {
            let a = account();
            let or = |s: &str| if s.is_empty() { "-".to_string() } else { s.to_string() };
            println!("E-mail: {}\nNazwa: {}\nOrganizacja: {}\nPlan: {}", or(&a.email), or(&a.name), or(&a.org), or(&a.plan));
            println!("Zalogowane: {}", if a.logged_in { "tak" } else { "nie" });
        }
        ClaudeCmd::Usage => {
            for w in crate::runtime::block_on(usage(paths, true))? {
                let reset = reset_text(w.resets_at.as_deref(), "pl", chrono::Local::now());
                println!("{}: {:.0}%  {reset}", window_label(&w.key, "pl"), w.percent);
            }
        }
        ClaudeCmd::Summary => println!("{}", summary(paths)),
        ClaudeCmd::Stats { details } => {
            let s = crate::tokens::stats::load(paths);
            println!("{}", if details { crate::tokens::spoken::details(&s) } else { crate::tokens::spoken::summary(&s) });
        }
        ClaudeCmd::Projects => {
            for p in projects() {
                println!("{:>9}  {:>4} sesji  {}  {}   [{}]", human(p.size), p.sessions, date(p.last), p.path, p.id);
            }
        }
        ClaudeCmd::Sessions { project } => {
            let p = find_project(&project)?;
            for s in sessions(paths, &p.id) {
                let flag = if s.current {
                    "BIEZACA ACHAI".to_string()
                } else if s.active {
                    "AKTYWNA".into()
                } else if !s.agent.is_empty() {
                    format!("agent {}", s.agent)
                } else {
                    String::new()
                };
                println!("{}  {}  {:>9}  {:>4} wiad.  {flag:13}  {}", s.id, date(s.mtime), human(s.size), s.messages, s.title);
            }
        }
        ClaudeCmd::Storage => {
            for (name, size) in storage() {
                println!("{:>9}  {name}", human(size));
            }
        }
        ClaudeCmd::DeleteSession { project, session } => {
            let p = find_project(&project)?;
            println!("Usunieto sesje {session} ({}).", human(delete_session(paths, &p.id, &session)?));
        }
        ClaudeCmd::DeleteProject { project } => {
            let p = find_project(&project)?;
            let freed = delete_project(paths, &p.id)?;
            println!("Usunieto sesje projektu {} ({}). Pliki projektu zostaly.", p.path, human(freed));
        }
        ClaudeCmd::Clean { days, apply } => {
            let (n, f, b) = clean(paths, days, apply);
            let verb = if apply { "Usunieto" } else { "Do usuniecia (dodaj --apply)" };
            println!("{verb}: {n} sesji starszych niz {days} dni i {f} innych plikow - {}.", human(b));
        }
    }
    Ok(0)
}
