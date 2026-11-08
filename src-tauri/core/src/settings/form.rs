//! What the settings window edits, read from config.json. Passwords and the Home Assistant
//! token never go to the window: it only learns whether one is saved, and an empty field when
//! saving means "unchanged".

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::Config;
use crate::paths::Paths;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppFlags {
    pub update_check: bool,
    pub update_install_on_quit: bool,
    pub close_to_tray: bool,
    pub show_console_on_prompt: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VoiceForm {
    pub microphone: String,
    pub local_device: String,
    pub rate: String,
    pub cast_enabled: bool,
    pub cast_name: String,
    pub cast_host: String,
    pub default_cast: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PhoneForm {
    pub enabled: bool,
    pub notify_service: String,
    pub push: bool,
    pub voice: bool,
    pub silent_override: bool,
    pub voice_hours: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HaForm {
    pub enabled: bool,
    pub url: String,
    #[serde(default)]
    pub token: String,
    pub has_token: bool,
    pub phone: PhoneForm,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccountForm {
    pub key: String,
    pub name: String,
    pub user: String,
    pub host: String,
    pub port: u16,
    pub security: String,
    pub display_name: String,
    pub signature: String,
    #[serde(default)]
    pub password: String,
    pub has_password: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MailForm {
    pub enabled: bool,
    pub important_hint: String,
    pub quiet_hours: String,
    pub delete_after_days: u32,
    pub important_senders: String,
    pub ignored_senders: String,
    pub digest_at: String,
    pub announce: Vec<String>,
    pub accounts: Vec<AccountForm>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SettingsForm {
    pub language: String,
    pub model: String,
    pub effort: String,
    pub theme: String,
    pub app: AppFlags,
    pub voice: VoiceForm,
    pub project_roots: Vec<String>,
    pub ha: HaForm,
    pub mail: MailForm,
    /// Achaja starts with Windows (the HKCU Run entry, handled by the window process).
    #[serde(default)]
    pub autostart: bool,
    /// config.json existed before: the window shows the sections, not the first-run wizard.
    #[serde(default)]
    pub editing: bool,
}

fn first_output(cfg: &Config, kind: &str) -> Option<(String, Value)> {
    cfg.get("voice.outputs")?.as_object()?.iter().find(|(_, v)| v["type"] == kind).map(|(n, v)| (n.clone(), v.clone()))
}

fn joined(cfg: &Config, path: &str) -> String {
    cfg.strs(path).join(", ")
}

pub fn load(paths: &Paths, raw: &Value, editing: bool) -> SettingsForm {
    let cfg = Config::new(raw.clone());
    let s = |v: &Value, k: &str| v[k].as_str().unwrap_or("").to_string();
    let local = first_output(&cfg, "local");
    let cast = first_output(&cfg, "cast");
    let flag = |k: &str| cfg.bool_or(&format!("app.{k}"), true);
    let token = crate::json::read_text(&paths.ha_token()).unwrap_or_default();
    let accounts = cfg
        .get("mail.accounts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter(|a| !s(a, "host").ends_with("example.com"))
        .map(|a| AccountForm {
            key: format!("{}@{}", s(a, "user"), s(a, "host")),
            name: s(a, "name"),
            user: s(a, "user"),
            host: s(a, "host"),
            port: a["port"].as_u64().unwrap_or(993) as u16,
            security: Some(s(a, "security")).filter(|x| !x.is_empty()).unwrap_or_else(|| "ssl".into()),
            display_name: s(a, "display_name"),
            signature: s(a, "signature"),
            password: String::new(),
            has_password: !s(a, "password").is_empty(),
        })
        .collect::<Vec<_>>();
    let announce = cfg.get("mail.announce").map(|_| cfg.strs("mail.announce")).unwrap_or_else(|| vec!["important".into(), "threat".into()]);
    SettingsForm {
        language: cfg.language(),
        model: cfg.str_or("achaja.model", "sonnet"),
        effort: cfg.str_or("achaja.effort", "low"),
        theme: cfg.str_or("app.theme", "auto"),
        app: AppFlags { update_check: flag("update_check"), update_install_on_quit: flag("update_install_on_quit"), close_to_tray: flag("close_to_tray"), show_console_on_prompt: flag("show_console_on_prompt") },
        voice: VoiceForm {
            microphone: cfg.str_or("wake.microphone", ""),
            local_device: local.as_ref().map(|(_, v)| s(v, "device")).unwrap_or_default(),
            rate: cfg.str_or("voice.rate", "+15%"),
            cast_enabled: cast.is_some(),
            cast_name: cast.as_ref().map(|(_, v)| s(v, "name")).unwrap_or_default(),
            cast_host: cast.as_ref().map(|(_, v)| s(v, "host")).unwrap_or_default(),
            default_cast: cast.as_ref().is_some_and(|(n, _)| cfg.str_or("voice.default_output", "") == *n),
        },
        project_roots: cfg.strs("agents.project_roots"),
        ha: HaForm {
            enabled: paths.mcp().exists(),
            url: cfg.str_or("home_assistant.url", ""),
            token: String::new(),
            has_token: !token.trim().is_empty(),
            phone: PhoneForm {
                enabled: cfg.bool_or("phone_alerts.enabled", false),
                notify_service: cfg.str_or("phone_alerts.notify_service", ""),
                push: cfg.bool_or("phone_alerts.push", true),
                voice: cfg.bool_or("phone_alerts.voice", true),
                silent_override: !cfg.str_or("phone_alerts.media_stream", "alarm_stream_max").is_empty(),
                voice_hours: cfg.str_or("phone_alerts.voice_hours", "08:00-22:00"),
            },
        },
        mail: MailForm {
            enabled: cfg.bool_or("mail.enabled", false) || accounts.is_empty(),
            important_hint: cfg.str_or("mail.important_hint", ""),
            quiet_hours: cfg.str_or("mail.quiet_hours", ""),
            delete_after_days: cfg.f64_or("mail.delete_after_days", 7.0) as u32,
            important_senders: joined(&cfg, "mail.important_senders"),
            ignored_senders: joined(&cfg, "mail.ignored_senders"),
            digest_at: joined(&cfg, "mail.digest_at"),
            announce,
            accounts,
        },
        editing,
        autostart: false,
    }
}

/// The Vosk model a language needs when it is not in data\models yet.
pub fn model_missing(paths: &Paths, raw: &Value, lang: &str) -> Option<String> {
    let name = raw["languages"][lang]["model"].as_str()?.to_string();
    (!paths.models().join(&name).is_dir()).then_some(name)
}
