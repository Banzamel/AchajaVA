//! What a project agent's Claude gets besides its prompt: the same reach as the Achaja session.
//! Claude in Chrome and Remote Control are on unless switched off (agents.chrome,
//! agents.remote_control); the claude.ai connectors (Gmail, Calendar, Drive...) and the user's
//! plugins load anyway from the account and ~/.claude, but a `-p` run connects them in the
//! background by default and the first turn could start without them, so it waits for them.

use crate::config::Config;

pub fn reach_args(cfg: &Config, name: &str) -> Vec<String> {
    let mut args = Vec::new();
    if cfg.bool_or("agents.chrome", true) {
        args.push("--chrome".to_string());
    }
    if cfg.bool_or("agents.remote_control", true) {
        // named after the project, so it can be told apart from the main "Achaja" session
        args.extend(["--remote-control".to_string(), format!("Agent: {name}")]);
    }
    args
}

/// Environment for the agent: the connectors are ready before the first turn.
pub const ENV: &[(&str, &str)] = &[("MCP_CONNECTION_NONBLOCKING", "false")];

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn chrome_and_remote_control_by_default() {
        let args = reach_args(&Config::new(json!({})), "vector");
        assert_eq!(args, ["--chrome", "--remote-control", "Agent: vector"]);
    }

    #[test]
    fn each_can_be_switched_off() {
        let cfg = Config::new(json!({"agents": {"chrome": false, "remote_control": false}}));
        assert!(reach_args(&cfg, "vector").is_empty());
    }
}
