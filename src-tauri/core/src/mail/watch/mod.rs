//! The background mail watcher (`Achaja.exe mail watch`, started by the window when mail is on):
//! IMAP IDLE on every account, rules and the model sort new mail, ads, spam and threats leave the
//! inbox on the server, and a spoken report comes when Achaja is not talking to the user. One
//! instance per computer. Log: state\mail\watch.log, results: state\mail\messages.json.

pub mod announce;
mod classify;
mod handle;
mod rules;
mod session;

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::config::{self, Config};
use crate::json::{append_line, now, read_json, write_json};
use crate::paths::Paths;
use crate::process::single_instance;

#[derive(Default)]
pub struct Pending {
    pub report: bool,
    pub texts: Vec<String>,
}

pub struct Ctx {
    pub paths: Paths,
    pub cfg: Config,
    pub pending: Mutex<Pending>,
    status_lock: Mutex<()>,
}

impl Ctx {
    pub fn log(&self, msg: &str) {
        let file = self.paths.state().join("mail").join("watch.log");
        if std::fs::metadata(&file).map(|m| m.len() > 1_000_000).unwrap_or(false) {
            let _ = std::fs::rename(&file, file.with_extension("log.1"));
        }
        let _ = append_line(&file, &format!("{} {msg}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")));
    }

    pub fn set_status(&self, account: &str, fields: Value) {
        let _guard = self.status_lock.lock();
        let file = self.paths.state().join("mail").join("status.json");
        let mut st: Value = read_json(&file, json!({}));
        if !st["accounts"].is_object() {
            st["accounts"] = json!({});
        }
        if !st["accounts"][account].is_object() {
            st["accounts"][account] = json!({});
        }
        if let (Some(target), Some(f)) = (st["accounts"][account].as_object_mut(), fields.as_object()) {
            for (k, v) in f {
                target.insert(k.clone(), v.clone());
            }
        }
        let _ = write_json(&file, &st);
    }

    pub fn queue_text(&self, text: String) {
        if let Ok(mut p) = self.pending.lock() {
            p.texts.push(text);
        }
    }

    pub fn lang(&self) -> &'static str {
        crate::mail::accounts::lang(&self.cfg)
    }
}

/// The watcher process; returns its exit code.
pub fn main(paths: &Paths) -> i32 {
    if !single_instance("AchajaMail") {
        return 0;
    }
    let cfg = config::load_or_default(paths);
    let ctx = Arc::new(Ctx { paths: paths.clone(), cfg: cfg.clone(), pending: Mutex::new(Pending::default()), status_lock: Mutex::new(()) });
    let accts = crate::mail::accounts::accounts(&cfg, false);
    if !cfg.bool_or("mail.enabled", false) || accts.is_empty() {
        ctx.log("Poczta wylaczona (mail.enabled) albo brak kont - koncze.");
        return 0;
    }
    let _ = write_json(&paths.state().join("mail").join("status.json"), &json!({"pid": std::process::id(), "started": now(), "accounts": {}}));
    ctx.log(&format!("Start nasluchu poczty: {}", accts.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", ")));
    let Ok(rt) = crate::runtime::multi_thread() else { return 1 };
    rt.block_on(async {
        for a in accts.clone() {
            if a.s("password").is_empty() {
                ctx.log(&format!("[{}] brak hasla w config.json - pomijam", a.name));
                ctx.set_status(&a.name, json!({"state": "error", "error": "brak hasla w config.json"}));
                continue;
            }
            tokio::spawn(session::watch_account(ctx.clone(), a));
        }
        announce::run(ctx.clone(), accts.len() > 1).await;
    });
    0
}
