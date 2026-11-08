//! Reading a project agent's stream-json output: progress lines for its log, the session id to
//! resume next time, and the final result turned into the report Achaja reads.

use serde_json::Value;

use crate::text::short;

pub enum Event {
    Session(String),
    Log(String),
    Result(Value),
    Nothing,
}

fn describe_tool(block: &Value) -> String {
    let name = block["name"].as_str().unwrap_or("");
    let input = &block["input"];
    for key in ["command", "file_path", "pattern", "path", "url", "description", "prompt"] {
        if let Some(v) = input.get(key) {
            let text = v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string());
            return format!("{name}: {}", short(&text, 160));
        }
    }
    format!("{name}: {}", short(&input.to_string(), 160))
}

/// One line of output -> what it means.
pub fn parse_line(line: &str) -> Vec<Event> {
    let line = line.trim();
    if line.is_empty() {
        return vec![Event::Nothing];
    }
    let Ok(msg) = serde_json::from_str::<Value>(line) else { return vec![Event::Log(format!("  {line}"))] };
    match msg["type"].as_str() {
        Some("system") if msg["subtype"] == "init" => msg["session_id"].as_str().map(|s| vec![Event::Session(s.to_string())]).unwrap_or_default(),
        Some("assistant") => msg["message"]["content"]
            .as_array()
            .map(|blocks| {
                blocks
                    .iter()
                    .filter_map(|b| match b["type"].as_str() {
                        Some("text") if !b["text"].as_str().unwrap_or("").trim().is_empty() => Some(Event::Log(format!("\n[agent] {}\n", b["text"].as_str().unwrap_or("").trim()))),
                        Some("tool_use") => Some(Event::Log(format!("  > {}", describe_tool(b)))),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default(),
        Some("result") => vec![Event::Result(msg)],
        _ => vec![Event::Nothing],
    }
}

pub fn report(name: &str, result: &Value) -> String {
    let meta = format!(
        "projekt: {name} | czas: {} s | tury: {} | blad: {} | koszt: {:.2} USD",
        (result["duration_ms"].as_f64().unwrap_or(0.0) / 1000.0).round(),
        result["num_turns"],
        if result["is_error"].as_bool().unwrap_or(false) { "True" } else { "False" },
        result["total_cost_usd"].as_f64().unwrap_or(0.0)
    );
    let body = result["result"].as_str().filter(|r| !r.is_empty()).unwrap_or("(brak tekstu)");
    let mut lines = vec![format!("# Wynik agenta projektu {name}"), meta, String::new(), body.to_string()];
    if let Some(denials) = result["permission_denials"].as_array().filter(|d| !d.is_empty()) {
        lines.push(String::new());
        lines.push("## Zablokowane akcje (wymagaja zgody uzytkownika)".into());
        for d in denials {
            lines.push(format!("- {}: {}", d["tool_name"].as_str().unwrap_or(""), short(&d["tool_input"].to_string(), 300)));
        }
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_progress_and_report() {
        let ev = parse_line(r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Bash","input":{"command":"npm test"}}]}}"#);
        assert!(matches!(&ev[0], Event::Log(l) if l == "  > Bash: npm test"));
        let ev = parse_line(r#"{"type":"system","subtype":"init","session_id":"abc"}"#);
        assert!(matches!(&ev[0], Event::Session(s) if s == "abc"));
        let r = serde_json::json!({"result": "Gotowe.", "num_turns": 3, "duration_ms": 61000, "permission_denials": [{"tool_name": "Bash", "tool_input": {"command": "git push"}}]});
        let text = report("shop", &r);
        assert!(text.contains("czas: 61 s") && text.contains("Zablokowane akcje") && text.contains("Gotowe."));
    }
}
