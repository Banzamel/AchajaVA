//! Quitting closes everything: the Claude session, the mail watcher, phone alerts, speech,
//! project agents and background jobs, with their children. A number from a state file may be
//! stale (after a restart it belongs to another program), so only Achaja's kind of process is
//! stopped.

use std::sync::atomic::Ordering;

use achaja_core::json::{read_text, read_value};
use achaja_core::paths::Paths;
use achaja_core::process::{kill_tree, pid_alive, process_name};

use super::state::AppState;

fn background_pids(p: &Paths) -> Vec<u32> {
    let mut pids: Vec<u32> = [
        read_value(&p.state().join("mail").join("status.json"))["pid"].as_u64(),
        read_value(&p.state().join("phone").join("run.json"))["pid"].as_u64(),
    ]
    .into_iter()
    .flatten()
    .map(|x| x as u32)
    .collect();
    if let Some(map) = read_value(&p.agents().join("running.json")).as_object() {
        pids.extend(map.values().filter_map(|r| r["pid"].as_u64()).map(|x| x as u32));
    }
    let mut files = vec![p.state_file("tts.pid")];
    files.extend(std::fs::read_dir(p.state().join("jobs")).into_iter().flatten().flatten().map(|e| e.path()).filter(|f| f.extension().is_some_and(|e| e == "pid")));
    pids.extend(files.iter().filter_map(|f| read_text(f).and_then(|t| t.trim().parse::<u32>().ok())));
    pids
}

fn ours(pid: u32) -> bool {
    process_name(pid).is_some_and(|n| ["achaja.exe", "claude.exe", "cmd.exe", "node.exe"].contains(&n.as_str()))
}

pub fn stop_everything(st: &AppState) {
    st.quitting.store(true, Ordering::Relaxed);
    let session = st.claude_pid();
    super::listener::stop(st);
    let me = std::process::id();
    for pid in background_pids(&st.paths).into_iter().chain(session) {
        if pid != me && pid_alive(pid) && ours(pid) {
            kill_tree(pid);
        }
    }
}
