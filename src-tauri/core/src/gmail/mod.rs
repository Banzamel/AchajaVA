//! `achaja gmail ...`: what the Claude Gmail connector lacks for the `gmail` skill — leaving a
//! mailing list (one-click POST of RFC 8058 when the sender supports it; a mailto is handed back
//! for Claude to send after the user's yes; a web link is only named, never opened), the user's
//! keep/junk sender rules and the log of clean-ups. Exit 3: Claude or the user has to act.

mod unsubscribe;

pub use unsubscribe::parse_header;

use clap::Subcommand;
use serde_json::{json, Value};

use crate::error::Result;
use crate::json::{read_json, write_json};
use crate::paths::Paths;

#[derive(Debug, Subcommand)]
pub enum GmailCmd {
    /// Wypisanie z listy mailingowej
    Unsubscribe {
        #[arg(long)]
        header: String,
        #[arg(long, default_value = "")]
        post: String,
        #[arg(long, default_value = "")]
        sender: String,
    },
    /// Zawsze zachowuj nadawce
    Keep { sender: String, #[arg(long, default_value = "")] note: String },
    /// Zawsze do kosza
    Junk { sender: String, #[arg(long, default_value = "")] note: String },
    /// Usun zasade
    Forget { sender: String },
    /// Zasady i historia wypisan
    Rules,
    /// Zapis wyniku przegladu
    Log {
        #[arg(long, default_value_t = 0)]
        trashed: u64,
        #[arg(long, default_value_t = 0)]
        spam: u64,
        #[arg(long, default_value_t = 0)]
        unsubscribed: u64,
        #[arg(long, default_value_t = 0)]
        kept: u64,
    },
}

pub(crate) fn file(paths: &Paths, name: &str) -> std::path::PathBuf {
    paths.state().join("gmail").join(name)
}

pub fn norm(sender: &str) -> String {
    let s = sender.trim().to_lowercase();
    match (s.find('<'), s.find('>')) {
        (Some(a), Some(b)) if b > a => s[a + 1..b].to_string(),
        _ => s,
    }
}

pub fn run(paths: &Paths, cmd: GmailCmd) -> Result<i32> {
    let rules_file = file(paths, "rules.json");
    match cmd {
        GmailCmd::Unsubscribe { header, post, sender } => crate::runtime::block_on(async { Ok(unsubscribe::unsubscribe(paths, &header, &post, &sender).await) }),
        cmd @ (GmailCmd::Keep { .. } | GmailCmd::Junk { .. } | GmailCmd::Forget { .. }) => {
            let (kind, sender, note) = match cmd {
                GmailCmd::Keep { sender, note } => ("keep", sender, note),
                GmailCmd::Junk { sender, note } => ("junk", sender, note),
                GmailCmd::Forget { sender } => ("forget", sender, String::new()),
                _ => return Ok(1),
            };
            let mut rules: Value = read_json(&rules_file, json!({"keep": {}, "junk": {}}));
            let key = norm(&sender);
            for k in ["keep", "junk"] {
                if let Some(m) = rules[k].as_object_mut() {
                    m.remove(&key);
                } else {
                    rules[k] = json!({});
                }
            }
            if kind != "forget" {
                rules[kind][&key] = json!({"note": note, "at": chrono::Local::now().format("%Y-%m-%d").to_string()});
            }
            write_json(&rules_file, &rules)?;
            println!("{}: {key}", match kind { "keep" => "Zachowuje", "junk" => "Do kosza", _ => "Usunieto zasade" });
            Ok(0)
        }
        GmailCmd::Rules => {
            let rules: Value = read_json(&rules_file, json!({}));
            for (kind, title) in [("keep", "ZAWSZE ZACHOWAJ"), ("junk", "ZAWSZE DO KOSZA")] {
                println!("{title}:");
                for (s, info) in rules[kind].as_object().cloned().unwrap_or_default() {
                    let note = info["note"].as_str().filter(|n| !n.is_empty()).map(|n| format!("  - {n}")).unwrap_or_default();
                    println!("  {s}{note}");
                }
            }
            println!("WYPISANIA (ostatnie 20):");
            for u in read_json::<Vec<Value>>(&file(paths, "unsubscribed.json"), Vec::new()).iter().take(20) {
                println!("  {}  {}  {}  {}", u["at"].as_str().unwrap_or(""), u["sender"].as_str().unwrap_or(""), u["how"].as_str().unwrap_or(""), if u["ok"] == json!(true) { "OK" } else { "czeka" });
            }
            if let Some(r) = read_json::<Vec<Value>>(&file(paths, "runs.json"), Vec::new()).first() {
                println!("OSTATNI PRZEGLAD: {r}");
            }
            Ok(0)
        }
        GmailCmd::Log { trashed, spam, unsubscribed, kept } => {
            let mut runs: Vec<Value> = read_json(&file(paths, "runs.json"), Vec::new());
            runs.insert(0, json!({"at": chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(), "trashed": trashed, "spam": spam, "unsubscribed": unsubscribed, "kept": kept}));
            runs.truncate(200);
            write_json(&file(paths, "runs.json"), &runs)?;
            println!("Zapisano.");
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_unsubscribe_header() {
        let (h, m) = parse_header("<https://x.com/u?a=1>, <mailto:u@x.com?subject=stop>");
        assert_eq!(h, ["https://x.com/u?a=1"]);
        assert_eq!(m, ["mailto:u@x.com?subject=stop"]);
        assert_eq!(norm("Shop <News@Shop.pl>"), "news@shop.pl");
    }
}
