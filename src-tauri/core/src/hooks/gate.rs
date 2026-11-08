//! PreToolUse hook: Achaja's gate. Irreversible and outward-facing actions (git push, deleting
//! files, sending mail, deploys, shutting down, locks and gates, installing an update, MCP tools
//! that send or delete) pass only in a turn the user started with their own yes (approval.json,
//! written by the prompt hook and cleared at the end of the turn). In background jobs only the
//! categories the job was allowed pass; "system" never. Any error denies.

use std::sync::OnceLock;

use regex::{Regex, RegexBuilder};
use serde_json::{json, Value};

use crate::json::{now, num};
use crate::paths::Paths;
use crate::state::{self, APPROVAL};

const APPROVAL_TTL: f64 = 30.0 * 60.0;
const ARGS: &str = r"[^\n|;&]*";

/// The Achaja tool called any way: "achaja.cmd plan", "$env:ACHAJA_EXE plan", "Achaja.exe plan",
/// or the module and script names of the Python versions.
fn cli(command: &str, module: &str, old: &str) -> String {
    format!(r#"(?:\bachaja(?:\.cmd|\.exe|_exe)?["'}}]?\s+{command}|\bachaja\.\w+\.{module}\b|\b{old}\.py["']?)\s+"#)
}

struct Rule {
    re: Regex,
    what: &'static str,
    category: &'static str,
    /// The rest of the line after the match must not contain this (mail drafts are fine).
    unless_after: Option<&'static str>,
}

fn rules() -> &'static [Rule] {
    static RULES: OnceLock<Vec<Rule>> = OnceLock::new();
    RULES.get_or_init(|| {
        let list: Vec<(String, &'static str, &'static str, Option<&'static str>)> = vec![
            (format!(r"\bgit\b{ARGS}\bpush\b"), "git push", "push", None),
            (format!(r"\bgit\b{ARGS}\breset\b{ARGS}--hard\b"), "git reset --hard", "delete", None),
            (format!(r"\bgit\b{ARGS}\bclean\b{ARGS}\s-\w*f"), "git clean", "delete", None),
            (format!(r"\bgit\b{ARGS}\bbranch\b{ARGS}\s-D\b"), "usuniecie galezi", "delete", None),
            (r"\b(?:remove-item|rm|rmdir|rd|del|erase|ri|shred|unlink)\b\s".into(), "usuwanie plikow", "delete", None),
            (r"\b(?:clear-recyclebin|format-volume|diskpart)\b|\bformat(?:\.com)?\s+[a-z]:".into(), "czyszczenie dysku", "system", None),
            (r"\b(?:stop-computer|restart-computer|shutdown)\b".into(), "wylaczenie komputera", "system", None),
            (r"\b(?:npm|pnpm|yarn)\s+publish\b|\btwine\s+upload\b|\bfirebase\s+deploy\b|\bvercel\b[^\n]*--prod|\bkubectl\s+(?:apply|delete)\b|\bterraform\s+(?:apply|destroy)\b|\bgh\s+release\s+create\b".into(), "deploy lub publikacja", "deploy", None),
            (cli("mail", "mail", "mail") + r"(?:send|reply)\b", "wyslanie maila", "mail", Some("--(?:draft|dry-run)")),
            (r"\bsend-mailmessage\b".into(), "wyslanie maila", "mail", None),
            (cli("gmail", "gmail", "gmail") + r"unsubscribe\b", "wypisanie z listy mailingowej", "mail", None),
            (cli("ha", "ha", "ha") + r"[^\n]*--confirmed", "zamek, brama, garaz lub alarm", "home", None),
            (cli("update", "update", "update") + r"install\b", "instalacja aktualizacji", "system", None),
            (cli("plan", "plan", "plan") + r"add\b[^\n]*--allow", "stala zgoda dla zadania w tle", "system", None),
            (cli("claude", "data", "claude_data") + r"(?:delete-session\b|delete-project\b|clean\b[^\n]*--apply)", "usuniecie sesji Claude", "delete", None),
            (cli("backup", "backup", "backup") + r"restore\b", "przywrocenie kopii zapasowej", "system", None),
        ];
        list.into_iter()
            .filter_map(|(p, what, category, unless)| {
                RegexBuilder::new(&p).case_insensitive(true).build().ok().map(|re| Rule { re, what, category, unless_after: unless })
            })
            .collect()
    })
}

fn mcp_risky(name: &str) -> bool {
    static RISKY: OnceLock<Option<Regex>> = OnceLock::new();
    static SAFE: OnceLock<Option<Regex>> = OnceLock::new();
    let risky = RISKY.get_or_init(|| RegexBuilder::new("send|trash|delete|remove|forward|reply|publish|merge|push|transfer|pay|purchase|order").case_insensitive(true).build().ok());
    let safe = SAFE.get_or_init(|| RegexBuilder::new("draft|^get|^list|^search|^read|^query").case_insensitive(true).build().ok());
    risky.as_ref().is_some_and(|r| r.is_match(name)) && !safe.as_ref().is_some_and(|s| s.is_match(name))
}

/// (what, category) of a risky action, or None.
pub fn risky(tool: &str, input: &Value) -> Option<(String, &'static str)> {
    if tool == "Bash" || tool == "PowerShell" {
        let command = input["command"].as_str().unwrap_or("");
        for rule in rules() {
            for m in rule.re.find_iter(command) {
                let rest = command[m.end()..].lines().next().unwrap_or("");
                let excused = rule.unless_after.and_then(|u| Regex::new(u).ok()).is_some_and(|u| u.is_match(rest));
                if !excused {
                    return Some((rule.what.to_string(), rule.category));
                }
            }
        }
        return None;
    }
    // Home Assistant's tools are watched by the home guard
    if tool.starts_with("mcp__") && !tool.starts_with("mcp__home-assistant__") {
        let name = tool.rsplit("__").next().unwrap_or("");
        if mcp_risky(name) {
            return Some((format!("narzedzie {name}"), "mcp"));
        }
    }
    None
}

pub fn deny(reason: String) -> Value {
    json!({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny", "permissionDecisionReason": reason}})
}

pub fn deny_session(what: &str) -> Value {
    deny(format!(
        "WYMAGA POTWIERDZENIA (bramka Achai): {what}. Nie obchodz tego innym poleceniem. Zapytaj uzytkownika glosowo (pytanie w sekcji [GŁOS]) i zakoncz ture; wykonasz to w nastepnej turze, jesli odpowie 'tak'."
    ))
}

fn deny_job(what: &str) -> Value {
    deny(format!(
        "ZADANIE W TLE NIE MA ZGODY: {what}. Nie obchodz tego innym poleceniem. Przygotuj szkic, jesli sie da (np. achaja.cmd mail reply ... --draft), i opisz w raporcie, co czeka na decyzje uzytkownika."
    ))
}

fn approved(paths: &Paths) -> bool {
    let a = state::read(paths, APPROVAL);
    a["yes"].as_bool().unwrap_or(false) && now() - num(&a, "at") < APPROVAL_TTL
}

/// The hook's answer for one tool call (None = let it run).
pub fn decide(paths: &Paths, data: &Value, session: bool, job_allow: Option<&str>) -> Option<Value> {
    if !session && job_allow.is_none() {
        return None;
    }
    let input = if data["tool_input"].is_object() { data["tool_input"].clone() } else { json!({}) };
    let (what, category) = risky(data["tool_name"].as_str().unwrap_or(""), &input)?;
    if let Some(allow) = job_allow {
        let allowed = allow.split(',').any(|a| a == category) && category != "system";
        return (!allowed).then(|| deny_job(&what));
    }
    (!approved(paths)).then(|| deny_session(&what))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bash(cmd: &str) -> Option<&'static str> {
        risky("Bash", &json!({ "command": cmd })).map(|(_, c)| c)
    }

    #[test]
    fn catches_risky_commands() {
        assert_eq!(bash("git push origin main"), Some("push"));
        assert_eq!(bash("Remove-Item -Recurse x"), Some("delete"));
        assert_eq!(bash(r#".\achaja.cmd mail reply 4 --text-file f.txt"#), Some("mail"));
        assert_eq!(bash(r#".\achaja.cmd mail reply 4 --text-file f.txt --draft"#), None);
        assert_eq!(bash(r#"& "$env:ACHAJA_EXE" ha call cover open_cover '{}' --confirmed"#), Some("home"));
        assert_eq!(bash("achaja plan add --job x --allow mail --daily 08:00"), Some("system"));
        assert_eq!(bash(r#"python -m achaja.claude.data clean --days 30 --apply"#), Some("delete"));
        assert_eq!(bash("achaja.cmd plan list"), None);
        assert_eq!(bash("Get-ChildItem"), None);
    }

    #[test]
    fn mcp_tools() {
        assert!(risky("mcp__claude_ai_Gmail__send_message", &json!({})).is_some());
        assert!(risky("mcp__claude_ai_Gmail__create_draft", &json!({})).is_none());
        assert!(risky("mcp__home-assistant__HassTurnOn", &json!({})).is_none());
    }

    #[test]
    fn jobs_pass_only_allowed_categories() {
        let dir = tempfile::tempdir().unwrap();
        let p = Paths::at(dir.path(), dir.path());
        let call = json!({"tool_name": "Bash", "tool_input": {"command": "achaja.cmd mail send --account a --to b --subject c --text-file d"}});
        assert!(decide(&p, &call, false, Some("mail")).is_none());
        assert!(decide(&p, &call, false, Some("")).is_some());
        assert!(decide(&p, &call, true, None).is_some());
        assert!(decide(&p, &call, false, None).is_none());
    }
}
