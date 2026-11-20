//! Prompts Claude Code writes itself: when a background command (e.g. `achaja.cmd agent run`)
//! ends, it starts a new turn with a `<task-notification>` block as the "user's" prompt, and the
//! UserPromptSubmit hook sees it like a dictated one. Such a turn is not the user speaking: the
//! chat shows a short note instead, nothing is acknowledged aloud and the gate is not touched.

use regex::Regex;

/// Is this prompt only Claude Code's own notifications (no words of the user)?
pub fn is_task_notification(prompt: &str) -> bool {
    prompt.trim_start().starts_with("<task-notification>")
}

fn tag<'a>(block: &'a str, name: &str) -> &'a str {
    let (open, close) = (format!("<{name}>"), format!("</{name}>"));
    block
        .split_once(open.as_str())
        .and_then(|(_, rest)| rest.split_once(close.as_str()))
        .map(|(inner, _)| inner.trim())
        .unwrap_or("")
}

/// One short line per notification for the chat, in the window's language.
pub fn notes(prompt: &str, lang: &str) -> Vec<String> {
    prompt.split("<task-notification>").skip(1).map(|block| note(block, lang)).collect()
}

fn note(block: &str, lang: &str) -> String {
    let status = tag(block, "status");
    let summary = tag(block, "summary");
    let ok = status == "completed" && (!summary.contains("exit code") || summary.contains("(exit code 0)"));
    let agent = Regex::new(r"agent\s+run\s+([^\s]+)")
        .ok()
        .and_then(|re| re.captures(summary).map(|c| c[1].trim_matches('"').to_string()));
    match (agent, ok, lang) {
        (Some(a), true, "en") => format!("Agent {a} has finished."),
        (Some(a), true, _) => format!("Agent {a} skończył pracę."),
        (Some(a), false, "en") => format!("Agent {a} ended with a problem ({status})."),
        (Some(a), false, _) => format!("Agent {a} zakończył się problemem ({status})."),
        (None, _, "en") => format!("Background task: {}", crate::text::truncate_chars(summary, 160)),
        (None, _, _) => format!("Zadanie w tle: {}", crate::text::truncate_chars(summary, 160)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AGENT: &str = "<task-notification>\n<task-id>b1</task-id>\n<status>completed</status>\n<summary>Background command \".\\achaja.cmd agent run vector --prompt-file \"state\\agents\\x.txt\"\" completed (exit code 0)</summary>\n</task-notification>";

    #[test]
    fn recognizes_notifications_only() {
        assert!(is_task_notification(&format!("  {AGENT}")));
        assert!(!is_task_notification("Achaja, co nowego w poczcie?"));
    }

    #[test]
    fn agent_runs_become_short_notes() {
        assert_eq!(notes(AGENT, "pl"), vec!["Agent vector skończył pracę."]);
        assert_eq!(notes(AGENT, "en"), vec!["Agent vector has finished."]);
        let failed = AGENT.replace("completed</status>", "failed</status>").replace("exit code 0", "exit code 1");
        assert_eq!(notes(&failed, "pl"), vec!["Agent vector zakończył się problemem (failed)."]);
    }

    #[test]
    fn other_commands_keep_their_summary() {
        let two = "<task-notification><status>completed</status><summary>Background command \"npm test\" completed (exit code 0)</summary></task-notification>\n<task-notification><status>killed</status><summary>x</summary></task-notification>";
        let n = notes(two, "pl");
        assert_eq!(n.len(), 2);
        assert_eq!(n[0], "Zadanie w tle: Background command \"npm test\" completed (exit code 0)");
    }
}
