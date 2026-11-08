//! The settings window's data: the form read from config.json and written back.

pub mod apply;
pub mod form;

pub use apply::apply;
pub use form::{load, model_missing, SettingsForm};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::Paths;
    use serde_json::json;

    #[test]
    fn round_trip_keeps_secrets_and_unknown_keys() {
        let dir = tempfile::tempdir().unwrap();
        let res = dir.path().join("res");
        std::fs::create_dir_all(res.join("claude")).unwrap();
        std::fs::write(res.join("claude/mcp.example.json"), r#"{"_comment": "x", "mcpServers": {"home-assistant": {"url": "u"}}}"#).unwrap();
        let p = Paths::at(&dir.path().join("data"), &res);
        std::fs::create_dir_all(&p.data).unwrap();
        let cfg = json!({"language": "pl", "custom": 1, "voice": {"outputs": {"słuchawki": {"type": "local", "device": ""}}},
            "mail": {"enabled": true, "accounts": [{"name": "firma", "user": "a@b.pl", "host": "imap.b.pl", "password": "tajne", "smtp_host": "smtp.b.pl"}]}}); // ggignore
        std::fs::write(p.config(), cfg.to_string()).unwrap();
        let mut form = load(&p, &cfg, true);
        assert!(form.mail.accounts[0].has_password && form.mail.accounts[0].password.is_empty());
        form.voice.cast_enabled = true;
        form.voice.cast_host = "10.0.0.5".into();
        form.voice.default_cast = true;
        form.ha.enabled = true;
        form.ha.url = "http://ha:8123/".into();
        form.ha.token = "tok".into();
        let saved = apply(&p, &form).unwrap();
        assert_eq!(saved["custom"], 1);
        assert_eq!(saved["mail"]["accounts"][0]["password"], "tajne"); // ggignore
        assert_eq!(saved["mail"]["accounts"][0]["smtp_host"], "smtp.b.pl");
        assert_eq!(saved["voice"]["default_output"], "głośnik");
        assert_eq!(std::fs::read_to_string(p.ha_token()).unwrap(), "tok");
        let mcp: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(p.mcp()).unwrap()).unwrap();
        assert_eq!(mcp["mcpServers"]["home-assistant"]["url"], "http://ha:8123/api/mcp");
        assert!(mcp.get("_comment").is_none());
    }
}
