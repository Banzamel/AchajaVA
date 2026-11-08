//! The command line of the hidden Achaja session (claude with its name, model, effort, remote
//! control and --continue) and Windows command-line quoting.

use std::path::Path;

use crate::config::Config;

pub fn session_args(cfg: &Config, resume: bool) -> Vec<String> {
    let mut args = vec![
        "--permission-mode".to_string(),
        cfg.str_or("agents.permission_mode", "auto"),
        "-n".into(),
        "Achaja".into(),
    ];
    let model = cfg.str_or("achaja.model", "");
    if !model.is_empty() {
        args.extend(["--model".into(), model]);
    }
    let effort = cfg.str_or("achaja.effort", "");
    if !effort.is_empty() {
        args.extend(["--effort".into(), effort]);
    }
    if cfg.bool_or("achaja.remote_control", true) {
        // the session can also be driven from the phone or claude.ai/code
        args.extend(["--remote-control".into(), "Achaja".into()]);
    }
    if resume {
        args.push("--continue".into());
    }
    args
}

/// One argument quoted the way the C runtime splits command lines.
pub fn quote(arg: &str) -> String {
    if !arg.is_empty() && !arg.contains([' ', '\t', '\n', '"']) {
        return arg.to_string();
    }
    let mut out = String::from("\"");
    let mut backslashes = 0;
    for c in arg.chars() {
        match c {
            '\\' => backslashes += 1,
            '"' => {
                out.push_str(&"\\".repeat(backslashes * 2 + 1));
                out.push('"');
                backslashes = 0;
            }
            _ => {
                out.push_str(&"\\".repeat(backslashes));
                out.push(c);
                backslashes = 0;
            }
        }
    }
    out.push_str(&"\\".repeat(backslashes * 2));
    out.push('"');
    out
}

/// (application, command line) for CreateProcess; a claude.cmd from npm runs through cmd.exe.
pub fn command_line(program: &Path, args: &[String]) -> (Option<String>, String) {
    let quoted: Vec<String> = args.iter().map(|a| quote(a)).collect();
    let prog = program.to_string_lossy().to_string();
    let ext = program.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if ext == "cmd" || ext == "bat" {
        let inner = format!("{} {}", quote(&prog), quoted.join(" "));
        return (None, format!("cmd.exe /d /s /c \"{inner}\""));
    }
    (Some(prog.clone()), format!("{} {}", quote(&prog), quoted.join(" ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_like_the_c_runtime() {
        assert_eq!(quote("plain"), "plain");
        assert_eq!(quote("two words"), "\"two words\"");
        assert_eq!(quote(r#"say "hi""#), r#""say \"hi\"""#);
        assert_eq!(quote(r"C:\dir with space\"), r#""C:\dir with space\\""#);
        assert_eq!(quote(""), "\"\"");
    }

    #[test]
    fn builds_session_args() {
        let cfg = Config::new(serde_json::json!({"achaja": {"model": "sonnet", "effort": "low"}}));
        let args = session_args(&cfg, true);
        assert_eq!(args[..4], ["--permission-mode", "auto", "-n", "Achaja"]);
        assert!(args.contains(&"--remote-control".to_string()));
        assert_eq!(args.last().map(String::as_str), Some("--continue"));
    }

    #[test]
    fn npm_claude_runs_through_cmd() {
        let (app, line) = command_line(Path::new(r"C:\npm\claude.cmd"), &["-n".into(), "Achaja".into()]);
        assert!(app.is_none());
        assert!(line.starts_with("cmd.exe /d /s /c \""));
    }
}
