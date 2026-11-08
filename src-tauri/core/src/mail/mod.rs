//! Mail: IMAP accounts watched in the background (ads, spam and threats leave the inbox, a spoken
//! report says what matters), and `achaja mail <command>` to read, correct and answer mail.
//! Only metadata stays on disk; mail content only ever reaches a model with no tools.

pub mod accounts;
pub mod actions;
pub mod commands;
pub mod find;
pub mod imap;
pub mod imap_ops;
pub mod log;
pub mod neutral;
pub mod parse;
pub mod report;
pub mod smtp;
pub mod utf7;
pub mod watch;

use clap::Subcommand;
use serde_json::Value;

use crate::json::read_json;
use crate::paths::Paths;
use crate::process::{kill_tree, pid_alive, spawn_self};

#[derive(Debug, Subcommand)]
pub enum MailCmd {
    /// Nasluch, konta, bledy, przeglad starej poczty
    Status,
    /// Raport do przeczytania na glos
    Summary {
        #[arg(long, default_value_t = 24.0)]
        hours: f64,
        #[arg(long)]
        backlog: bool,
    },
    /// Ostatnio sklasyfikowane wiadomosci
    List {
        #[arg(long, value_parser = clap::builder::PossibleValuesParser::new(accounts::CATEGORIES))]
        category: Option<String>,
        #[arg(long, default_value_t = 24.0)]
        hours: f64,
        #[arg(long)]
        account: Option<String>,
        #[arg(long)]
        all: bool,
    },
    /// Pelna tresc (nie oznacza jako przeczytanej)
    Show { id: u64 },
    /// Poprawka klasyfikacji / przywrocenie do skrzynki
    Move { id: u64, target: String },
    /// Ponowny przeglad poczty w skrzynce
    Rescan {
        #[arg(long)]
        account: Option<String>,
        #[arg(long)]
        days: Option<u64>,
        #[arg(long)]
        all: bool,
    },
    /// Odpowiedz na wiadomosc
    Reply {
        id: u64,
        #[arg(long)]
        text_file: String,
        #[arg(long)]
        all: bool,
        #[arg(long)]
        draft: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// Nowa wiadomosc
    Send {
        #[arg(long)]
        account: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        subject: String,
        #[arg(long)]
        text_file: String,
        #[arg(long)]
        draft: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// Logowanie IMAP/SMTP kazdego konta
    Test,
    Start,
    Stop,
    Restart,
    /// Nasluch w tle (uruchamia go Achaja)
    #[command(hide = true)]
    Watch,
}

/// The watcher's PID when it runs.
pub fn watch_pid(paths: &Paths) -> Option<u32> {
    let st: Value = read_json(&paths.state().join("mail").join("status.json"), Value::Null);
    st["pid"].as_u64().map(|p| p as u32).filter(|p| pid_alive(*p))
}

pub fn start_watch(paths: &Paths) -> String {
    if watch_pid(paths).is_some() {
        return "Nasluch poczty juz dziala.".into();
    }
    let _ = spawn_self(&["mail", "watch"], &paths.data);
    std::thread::sleep(std::time::Duration::from_secs(3));
    if watch_pid(paths).is_some() {
        "Uruchomiono nasluch poczty.".into()
    } else {
        "Nasluch nie wystartowal - sprawdz state\\mail\\watch.log i mail.enabled w config.json.".into()
    }
}

pub fn stop_watch(paths: &Paths) -> bool {
    match watch_pid(paths) {
        Some(pid) => {
            kill_tree(pid);
            true
        }
        None => false,
    }
}

/// IMAP and SMTP login of one account: [(ok, description)].
pub async fn test_account(a: &accounts::Account) -> Vec<(bool, String)> {
    let mut out = Vec::new();
    match imap::connect(a).await {
        Ok(mut c) => {
            let folder = c.server_folder(&a.folder()).await;
            let info = c.session.examine(imap::quote(&folder)).await;
            let unseen = c.session.search("UNSEEN").await.map(|s| s.len()).unwrap_or(0);
            let idle = if c.has("IDLE") { "tak".to_string() } else { format!("nie (odpytywanie co {} s)", a.n("poll_seconds", 120.0)) };
            let exists = info.map(|m| m.exists).unwrap_or(0);
            out.push((true, format!("odbieranie: {exists} wiadomosci w {} (nieprzeczytane: {unseen}) | IDLE: {idle} | MOVE: {}", a.folder(), if c.has("MOVE") { "tak" } else { "nie" })));
            c.logout().await;
        }
        Err(e) => out.push((false, format!("odbieranie (IMAP): {e}"))),
    }
    match smtp::test(a).await {
        Ok(d) => out.push((true, format!("wysylanie: {d}"))),
        Err(e) => out.push((false, format!("wysylanie (SMTP - odpisywanie nie zadziala): {e}"))),
    }
    out
}
