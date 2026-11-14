//! A read-only `claude -p` over one claude.ai connector (Gmail, Google Calendar): only the listed
//! reading tools allowed, every writing tool and the other connectors refused, no hooks, no saved
//! session, the answer forced into a JSON schema. The stream's first event tells which
//! connectors are connected; that status is also probed alone (without a model call) for the
//! settings, which grey out what the account has not connected.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::process::Stdio;

use serde_json::{json, Value};

use crate::json::{now, read_json, write_json};
use crate::paths::Paths;
use crate::process::{claude_command, find_claude, kill_tree};

pub const GMAIL: &str = "mcp__claude_ai_Gmail__";
pub const CALENDAR: &str = "mcp__claude_ai_Google_Calendar__";
const PROBE_TTL: f64 = 600.0;

/// "gmail" / "calendar" -> the status Claude Code reported ("connected", "needs-auth", ...).
pub type Statuses = BTreeMap<String, String>;

pub struct Answer {
    pub statuses: Statuses,
    /// The schema's JSON (None when the run failed).
    pub output: Option<Value>,
    /// The final result event (tokens for the ledger).
    pub result: Value,
}

fn key_of(server: &str) -> Option<&'static str> {
    let s = server.to_lowercase();
    if s.contains("gmail") {
        Some("gmail")
    } else if s.contains("calendar") {
        Some("calendar")
    } else {
        None
    }
}

/// The statuses of the init event and the final result of a stream-json output.
pub fn parse_stream(raw: &str) -> (Statuses, Value) {
    let (mut statuses, mut result) = (Statuses::new(), Value::Null);
    for line in raw.lines() {
        let Ok(ev) = serde_json::from_str::<Value>(line) else { continue };
        if ev["type"] == "system" && ev["subtype"] == "init" {
            for s in ev["mcp_servers"].as_array().cloned().unwrap_or_default() {
                if let Some(k) = key_of(s["name"].as_str().unwrap_or("")) {
                    statuses.insert(k.into(), s["status"].as_str().unwrap_or("").into());
                }
            }
        } else if ev["type"] == "result" {
            result = ev;
        }
    }
    (statuses, result)
}

fn command(model: &str, system: &str, allowed: &[String], denied: &[String]) -> Option<std::process::Command> {
    let mut cmd = claude_command(&find_claude()?);
    cmd.args(["-p", "--model", model, "--tools", "", "--permission-mode", "dontAsk", "--allowedTools", &allowed.join(","), "--disallowedTools", &denied.join(",")]);
    cmd.args(["--setting-sources", "", "--settings", "{\"disableAllHooks\": true}", "--no-session-persistence", "--output-format", "stream-json", "--verbose"]);
    cmd.args(["--system-prompt", system]);
    cmd.env("ENABLE_TOOL_SEARCH", "false").env("MCP_CONNECTION_NONBLOCKING", "false");
    cmd.current_dir(std::env::temp_dir()).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
    Some(cmd)
}

/// Runs the model over the connector and returns its JSON answer.
pub fn ask(model: &str, system: &str, prompt: &str, schema: &Value, allowed: &[String], denied: &[String]) -> Option<Answer> {
    let mut cmd = command(model, system, allowed, denied)?;
    cmd.args(["--json-schema", &schema.to_string()]);
    let out = cmd
        .spawn()
        .and_then(|mut c| {
            if let Some(mut i) = c.stdin.take() {
                i.write_all(prompt.as_bytes())?;
            }
            c.wait_with_output()
        })
        .ok()?;
    let (statuses, result) = parse_stream(&String::from_utf8_lossy(&out.stdout));
    let output = (result.is_object() && result["is_error"] != json!(true)).then(|| result["structured_output"].clone()).filter(|v| !v.is_null());
    Some(Answer { statuses, output, result })
}

/// The connectors' statuses without a model call: the init event comes before any request, and
/// the process is stopped right after it. Cached for ten minutes unless `refresh`.
pub fn probe(paths: &Paths, refresh: bool) -> Statuses {
    let file = paths.state().join("phone").join("connectors.json");
    let cached: Value = read_json(&file, json!({}));
    if !refresh && now() - cached["at"].as_f64().unwrap_or(0.0) < PROBE_TTL {
        if let Ok(s) = serde_json::from_value::<Statuses>(cached["statuses"].clone()) {
            return s;
        }
    }
    let Some(mut cmd) = command("haiku", "Reply OK.", &[], &[]) else { return Statuses::new() };
    let Ok(mut child) = cmd.spawn() else { return Statuses::new() };
    if let Some(mut i) = child.stdin.take() {
        let _ = i.write_all(b"OK"); // dropped here: stdin closes
    }
    let mut statuses = Statuses::new();
    if let Some(out) = child.stdout.take() {
        for line in BufReader::new(out).lines().map_while(std::result::Result::ok).take(50) {
            if line.contains("\"subtype\":\"init\"") {
                statuses = parse_stream(&line).0;
                break;
            }
        }
    }
    kill_tree(child.id());
    let _ = child.wait();
    let _ = write_json(&file, &json!({"at": now(), "statuses": statuses}));
    statuses
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_statuses_and_result() {
        let raw = [
            r#"{"type":"system","subtype":"init","mcp_servers":[{"name":"claude.ai Gmail","status":"connected"},{"name":"claude.ai Google Calendar","status":"needs-auth"},{"name":"other","status":"connected"}]}"#,
            r#"{"type":"result","is_error":false,"structured_output":{"items":[]},"modelUsage":{}}"#,
        ]
        .join("\n");
        let (s, r) = parse_stream(&raw);
        assert_eq!(s.get("gmail").map(String::as_str), Some("connected"));
        assert_eq!(s.get("calendar").map(String::as_str), Some("needs-auth"));
        assert_eq!(s.len(), 2);
        assert_eq!(r["structured_output"]["items"], json!([]));
    }
}
