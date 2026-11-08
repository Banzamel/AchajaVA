//! Writing the settings window's form back: config.json (keeping every key the form does not
//! show), the Home Assistant token, .mcp.json and the MCP server's switch in
//! .claude\settings.local.json.

use serde_json::{json, Map, Value};

use super::form::SettingsForm;
use crate::error::Result;
use crate::json::{read_value, write_json, write_text};
use crate::paths::Paths;

fn split(list: &str) -> Vec<String> {
    list.replace('\n', ",").split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

fn obj<'a>(v: &'a mut Value, key: &str) -> &'a mut Map<String, Value> {
    if !v[key].is_object() {
        v[key] = json!({});
    }
    match v.get_mut(key) {
        Some(Value::Object(m)) => m,
        _ => unreachable!("set to an object above"),
    }
}

fn output_names(lang: &str) -> (&'static str, &'static str) {
    if lang == "pl" {
        ("słuchawki", "głośnik")
    } else {
        ("headphones", "speaker")
    }
}

fn apply_voice(cfg: &mut Value, f: &SettingsForm) {
    obj(cfg, "wake").insert("microphone".into(), json!(f.voice.microphone));
    let voice = obj(cfg, "voice");
    if !f.voice.rate.trim().is_empty() {
        voice.insert("rate".into(), json!(f.voice.rate.trim()));
    }
    let mut outputs = voice.get("outputs").and_then(Value::as_object).cloned().unwrap_or_default();
    let (local_default, cast_default) = output_names(&f.language);
    let local_name = outputs.iter().find(|(_, v)| v["type"] == "local").map(|(n, _)| n.clone()).unwrap_or_else(|| local_default.to_string());
    let local = outputs.entry(local_name.clone()).or_insert_with(|| json!({"type": "local", "aliases": []}));
    local["device"] = json!(f.voice.local_device);
    let mut cast_name = outputs.iter().find(|(_, v)| v["type"] == "cast").map(|(n, _)| n.clone());
    if f.voice.cast_enabled && !f.voice.cast_host.trim().is_empty() {
        let name = cast_name.clone().unwrap_or_else(|| cast_default.to_string());
        let cast = outputs.entry(name.clone()).or_insert_with(|| json!({"type": "cast", "aliases": []}));
        cast["name"] = json!(f.voice.cast_name);
        cast["host"] = json!(f.voice.cast_host.trim());
        cast_name = Some(name);
    } else if let Some(n) = cast_name.take() {
        outputs.remove(&n);
    }
    let default = match (&cast_name, f.voice.default_cast) {
        (Some(c), true) => c.clone(),
        _ => local_name,
    };
    voice.insert("outputs".into(), Value::Object(outputs));
    voice.insert("default_output".into(), json!(default));
}

fn apply_ha(paths: &Paths, cfg: &mut Value, f: &SettingsForm) -> Result<()> {
    if !f.ha.enabled {
        return Ok(());
    }
    let url = f.ha.url.trim().trim_end_matches('/').to_string();
    obj(cfg, "home_assistant").insert("url".into(), json!(url));
    if !f.ha.token.trim().is_empty() {
        write_text(&paths.ha_token(), f.ha.token.trim())?;
    }
    let mut mcp = read_value(&paths.bundled("claude/mcp.example.json"));
    if let Some(m) = mcp.as_object_mut() {
        m.remove("_comment");
    }
    mcp["mcpServers"]["home-assistant"]["url"] = json!(format!("{url}/api/mcp"));
    write_text(&paths.mcp(), &(serde_json::to_string_pretty(&mcp)? + "\n"))?;
    let local_file = paths.claude_dir().join("settings.local.json");
    let mut local = read_value(&local_file);
    if !local.is_object() {
        local = json!({});
    }
    let mut servers: Vec<String> = local["enabledMcpjsonServers"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default();
    servers.push("home-assistant".into());
    servers.sort();
    servers.dedup();
    local["enabledMcpjsonServers"] = json!(servers);
    write_json(&local_file, &local)?;
    let p = &f.ha.phone;
    let phone = obj(cfg, "phone_alerts");
    for (k, v) in [
        ("enabled", json!(p.enabled)),
        ("notify_service", json!(p.notify_service.trim())),
        ("push", json!(p.push)),
        ("voice", json!(p.voice)),
        ("media_stream", json!(if p.silent_override { "alarm_stream_max" } else { "" })),
        ("voice_hours", json!(p.voice_hours.trim())),
    ] {
        phone.insert(k.into(), v);
    }
    Ok(())
}

fn apply_mail(cfg: &mut Value, f: &SettingsForm) {
    let old: Vec<Value> = cfg["mail"]["accounts"].as_array().cloned().unwrap_or_default();
    let key = |a: &Value| format!("{}@{}", a["user"].as_str().unwrap_or(""), a["host"].as_str().unwrap_or(""));
    let mut accounts = Vec::new();
    for row in f.mail.accounts.iter().filter(|r| !r.user.trim().is_empty() && !r.host.trim().is_empty()) {
        let prev = old.iter().find(|a| key(a) == row.key).cloned().unwrap_or(json!({}));
        let mut acct = prev.as_object().cloned().unwrap_or_default(); // keeps smtp_*, handling...
        for (k, v) in [("name", &row.name), ("host", &row.host), ("user", &row.user), ("display_name", &row.display_name), ("signature", &row.signature)] {
            acct.insert(k.into(), json!(v.trim()));
        }
        acct.insert("port".into(), json!(if row.port > 0 { row.port } else { 993 }));
        acct.insert("security".into(), json!(if row.security.is_empty() { "ssl" } else { &row.security }));
        let pass = if row.password.is_empty() { prev["password"].as_str().unwrap_or("").to_string() } else { row.password.clone() };
        acct.insert("password".into(), json!(pass));
        for k in ["display_name", "signature"] {
            if acct.get(k).and_then(Value::as_str) == Some("") {
                acct.remove(k);
            }
        }
        accounts.push(Value::Object(acct));
    }
    let m = &f.mail;
    let mail = obj(cfg, "mail");
    for (k, v) in [
        ("enabled", json!(m.enabled && !accounts.is_empty())),
        ("important_hint", json!(m.important_hint)),
        ("quiet_hours", json!(m.quiet_hours)),
        ("delete_after_days", json!(m.delete_after_days)),
        ("important_senders", json!(split(&m.important_senders))),
        ("ignored_senders", json!(split(&m.ignored_senders))),
        ("digest_at", json!(split(&m.digest_at))),
        ("announce", json!(m.announce.iter().filter(|c| crate::mail::accounts::CATEGORIES.contains(&c.as_str())).collect::<Vec<_>>())),
        ("accounts", json!(accounts)),
    ] {
        mail.insert(k.into(), v);
    }
}

pub fn apply(paths: &Paths, f: &SettingsForm) -> Result<Value> {
    let mut cfg = crate::config::load_raw(paths)?;
    if !cfg.is_object() {
        cfg = json!({});
    }
    cfg["language"] = json!(f.language);
    let achaja = obj(&mut cfg, "achaja");
    for (k, v) in [("model", &f.model), ("effort", &f.effort)] {
        if !v.is_empty() {
            achaja.insert(k.into(), json!(v));
        }
    }
    let app = obj(&mut cfg, "app");
    for (k, v) in [("update_check", f.app.update_check), ("update_install_on_quit", f.app.update_install_on_quit), ("close_to_tray", f.app.close_to_tray), ("show_console_on_prompt", f.app.show_console_on_prompt)] {
        app.insert(k.into(), json!(v));
    }
    if ["auto", "light", "dark"].contains(&f.theme.as_str()) {
        app.insert("theme".into(), json!(f.theme));
    }
    apply_voice(&mut cfg, f);
    obj(&mut cfg, "agents").insert("project_roots".into(), json!(f.project_roots.iter().map(|r| r.trim()).filter(|r| !r.is_empty()).collect::<Vec<_>>()));
    apply_ha(paths, &mut cfg, f)?;
    apply_mail(&mut cfg, f);
    crate::config::save_raw(paths, &cfg)?;
    Ok(cfg)
}
