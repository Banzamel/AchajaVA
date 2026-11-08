//! `Achaja.exe <command> ...`: one entry for every tool of the Achaja session (also through
//! data\achaja.cmd) and for the session hooks. Without a command the window starts instead.

mod console;
mod small;

use clap::{Parser, Subcommand};

use crate::paths::paths;

pub const COMMANDS: &[&str] = &[
    "plan", "jobs", "mail", "gmail", "phone", "ha", "agent", "claude", "backup", "update", "output", "speak", "hook", "devices", "audio", "help", "--help", "-h",
];

#[derive(Debug, Parser)]
#[command(name = "achaja", version = crate::VERSION, about = "Achaja - narzedzia asystentki glosowej (bez polecenia: okno Achai)")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Przypomnienia, polecenia o czasie i zadania w tle
    Plan {
        #[command(subcommand)]
        cmd: small::PlanCmd,
    },
    /// Wyniki zadan w tle
    Jobs {
        #[command(subcommand)]
        cmd: small::JobsCmd,
    },
    /// Poczta: raport, odczyt, odpowiedzi, przywracanie
    Mail {
        #[command(subcommand)]
        cmd: crate::mail::MailCmd,
    },
    /// Porzadki w Gmailu (konektor Claude)
    Gmail {
        #[command(subcommand)]
        cmd: crate::gmail::GmailCmd,
    },
    /// Wazna poczta na telefon
    Phone {
        #[command(subcommand)]
        cmd: crate::phone::PhoneCmd,
    },
    /// Home Assistant: stan, klimatyzacja, uslugi, obszary
    Ha {
        #[arg(long, global = false)]
        dry_run: bool,
        #[arg(long)]
        confirmed: bool,
        #[command(subcommand)]
        cmd: crate::homeassistant::HaCmd,
    },
    /// Agenci projektow
    Agent {
        #[command(subcommand)]
        cmd: crate::agents::AgentCmd,
    },
    /// Konto, limity planu i sesje Claude Code
    Claude {
        #[command(subcommand)]
        cmd: crate::claude::cli::ClaudeCmd,
    },
    /// Kopie zapasowe
    Backup {
        #[command(subcommand)]
        cmd: crate::backup::BackupCmd,
    },
    /// Aktualizacje
    Update {
        #[command(subcommand)]
        cmd: crate::update::UpdateCmd,
    },
    /// Wyjscie glosu (sluchawki, glosnik)
    Output {
        #[command(subcommand)]
        cmd: Option<small::OutputCmd>,
    },
    /// Przeczytaj tekst na glos
    Speak {
        #[arg(long)]
        file: Option<String>,
        text: Vec<String>,
    },
    /// Hook sesji Claude Code (event, stop, gate, home-guard)
    #[command(hide = true)]
    Hook { name: String },
    /// Mikrofony i wyjscia audio
    Devices,
    /// Mikrofon/glosnik dla jednego programu: audio <pid> input|output <nazwa|default>, albo audio list
    Audio { args: Vec<String> },
}

/// Runs a command when the first argument names one; None = start the window.
pub fn try_run(args: &[String]) -> Option<i32> {
    let first = args.get(1)?;
    if !COMMANDS.contains(&first.as_str()) {
        return None;
    }
    console::attach();
    let cli = match Cli::try_parse_from(args) {
        Ok(c) => c,
        Err(e) => {
            let _ = e.print();
            return Some(if e.use_stderr() { 2 } else { 0 });
        }
    };
    let p = paths();
    let _ = crate::paths::ensure_dirs(p);
    let result = match cli.cmd {
        Cmd::Plan { cmd } => small::plan(p, cmd),
        Cmd::Jobs { cmd } => small::jobs(p, cmd),
        Cmd::Mail { cmd } => crate::mail::commands::run(p, cmd),
        Cmd::Gmail { cmd } => crate::gmail::run(p, cmd),
        Cmd::Phone { cmd } => crate::phone::cli(p, cmd),
        Cmd::Ha { dry_run, confirmed, cmd } => crate::homeassistant::run(p, cmd, dry_run, confirmed),
        Cmd::Agent { cmd } => crate::agents::cli(p, cmd),
        Cmd::Claude { cmd } => crate::claude::cli::run(p, cmd),
        Cmd::Backup { cmd } => crate::backup::run(p, cmd),
        Cmd::Update { cmd } => crate::update::run(p, cmd),
        Cmd::Output { cmd } => small::output(p, cmd),
        Cmd::Speak { file, text } => small::speak(p, file, text),
        Cmd::Hook { name } => Ok(crate::hooks::run(p, &name)),
        Cmd::Devices => small::devices(),
        Cmd::Audio { args } => small::audio(&args),
    };
    Some(match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("BLAD: {e}");
            1
        }
    })
}
