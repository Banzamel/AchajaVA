//! Claude Code hooks of the Achaja session (`Achaja.exe hook <name>`, wired in
//! data\.claude\settings.json): the prompt and notification events, the spoken answer, the
//! confirmation gate and the guard for locks and gates. Each reads the event JSON on stdin and
//! answers on stdout; they must be quick and must never let a gated action through by failing.

pub mod event;
pub mod gate;
pub mod home_guard;
pub mod stop;
pub mod yes;

use std::io::Read;

use serde_json::Value;

use crate::paths::Paths;

fn read_stdin() -> Value {
    let mut buf = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut buf);
    serde_json::from_slice(crate::json::strip_bom(&buf)).unwrap_or(Value::Null)
}

fn emit(value: &Value) {
    println!("{value}");
}

/// Runs one hook; the process exit code.
pub fn run(paths: &Paths, name: &str) -> i32 {
    let data = read_stdin();
    let session = std::env::var_os("ACHAJA_SESSION").is_some();
    let job = std::env::var("ACHAJA_JOB").ok();
    match name {
        "event" if session => event::handle(paths, &data),
        "stop" => {
            if let Some(out) = stop::handle(paths, &data, session, job.is_some()) {
                emit(&out);
            }
        }
        // a failure of the check itself must refuse, never let the action through
        "gate" => {
            let allow = job.as_ref().map(|_| std::env::var("ACHAJA_JOB_ALLOW").unwrap_or_default());
            let out = std::panic::catch_unwind(|| gate::decide(paths, &data, session, allow.as_deref()))
                .unwrap_or_else(|_| Some(gate::deny_session("blad sprawdzania")));
            if let Some(out) = out {
                emit(&out);
            }
        }
        "home-guard" => {
            let out = std::panic::catch_unwind(|| home_guard::decide(paths, &data))
                .unwrap_or_else(|_| Some(home_guard::deny_home("blad sprawdzania")));
            if let Some(out) = out {
                emit(&out);
            }
        }
        // started by the Stop hook: counts the answered command once it is in the transcript
        "tokens" => crate::tokens::transcript::record_pending(paths),
        _ => {}
    }
    0
}
