//! The signed-in Claude account and the plan's usage windows (the same reading as /usage in
//! Claude Code, with Claude Code's own token from ~/.claude/.credentials.json; the token is never
//! printed or stored). The last answer is kept for a minute.

use serde::Serialize;
use serde_json::{json, Value};

use super::trust::{claude_home, global_config};
use crate::error::{AppError, Result};
use crate::json::{now, num, read_json, read_value, write_json};
use crate::paths::Paths;

const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const USAGE_TTL: f64 = 60.0;

#[derive(Debug, Clone, Default, Serialize)]
pub struct Account {
    pub email: String,
    pub name: String,
    pub org: String,
    pub plan: String,
    pub logged_in: bool,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq)]
pub struct UsageWindow {
    pub key: String,
    pub percent: f64,
    pub resets_at: Option<String>,
}

fn credentials() -> Value {
    read_value(&claude_home().join(".credentials.json")).get("claudeAiOauth").cloned().unwrap_or(Value::Null)
}

/// "Max 5x", "Pro"... from the subscription type and the rate-limit tier.
pub fn plan_name(subscription: &str, tier: &str) -> String {
    let base = match subscription.to_lowercase().as_str() {
        "" => return String::new(),
        "max" => "Max".to_string(),
        "pro" => "Pro".into(),
        "team" => "Team".into(),
        "enterprise" => "Enterprise".into(),
        "free" => "Free".into(),
        other => crate::text::cap(other),
    };
    let times = regex::Regex::new(r"max_(\d+)x").ok().and_then(|r| r.captures(tier).map(|c| c[1].to_string()));
    match times {
        Some(n) if base == "Max" => format!("Max {n}x"),
        _ => base,
    }
}

pub fn account() -> Account {
    let acct = read_value(&global_config()).get("oauthAccount").cloned().unwrap_or(Value::Null);
    let cred = credentials();
    let s = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let sub = if s(&cred, "subscriptionType").is_empty() { s(&acct, "subscriptionType") } else { s(&cred, "subscriptionType") };
    Account {
        email: s(&acct, "emailAddress"),
        name: s(&acct, "displayName"),
        org: s(&acct, "organizationName"),
        plan: plan_name(&sub, &s(&cred, "rateLimitTier")),
        logged_in: !s(&cred, "accessToken").is_empty(),
    }
}

/// The 5-hour and 7-day windows (and their per-model variants) from the usage answer.
pub fn parse_usage(raw: &Value) -> Vec<UsageWindow> {
    let mut windows: Vec<UsageWindow> = raw
        .as_object()
        .map(|o| {
            o.iter()
                .filter(|(k, v)| (k.starts_with("five_hour") || k.starts_with("seven_day")) && v["utilization"].is_number())
                .map(|(k, v)| UsageWindow {
                    key: k.clone(),
                    percent: v["utilization"].as_f64().unwrap_or(0.0),
                    resets_at: v["resets_at"].as_str().map(str::to_string),
                })
                .collect()
        })
        .unwrap_or_default();
    let order = |k: &str| if k == "five_hour" { 0 } else if k == "seven_day" { 1 } else { 2 };
    windows.sort_by(|a, b| (order(&a.key), &a.key).cmp(&(order(&b.key), &b.key)));
    windows
}

pub async fn usage(paths: &Paths, force: bool) -> Result<Vec<UsageWindow>> {
    let cache_file = paths.state_file("claude_usage.json");
    let cache = read_value(&cache_file);
    if !force && now() - num(&cache, "at") < USAGE_TTL {
        if let Ok(w) = serde_json::from_value::<Vec<UsageWindow>>(cache["windows"].clone()) {
            return Ok(w);
        }
    }
    let cred = credentials();
    let token = cred["accessToken"].as_str().unwrap_or("");
    if token.is_empty() {
        return Err(AppError::msg("Brak zalogowanego konta claude.ai w Claude Code (uruchom claude i zaloguj sie)."));
    }
    if num(&cred, "expiresAt") > 0.0 && num(&cred, "expiresAt") / 1000.0 < now() {
        return Err(AppError::msg("Token Claude Code wygasl - odswiezy sie przy najblizszym uzyciu claude."));
    }
    let resp = crate::runtime::http(15)?
        .get(USAGE_URL)
        .bearer_auth(token)
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| AppError::msg(format!("Limity: brak polaczenia ({e}).")))?;
    if !resp.status().is_success() {
        return Err(AppError::msg(format!("Limity: HTTP {} z api.anthropic.com.", resp.status().as_u16())));
    }
    let windows = parse_usage(&resp.json::<Value>().await?);
    let _ = write_json(&cache_file, &json!({ "at": now(), "windows": windows }));
    Ok(windows)
}

/// The usage cached on disk (the window shows it while a new reading is on its way).
pub fn cached_usage(paths: &Paths) -> Vec<UsageWindow> {
    read_json::<Value>(&paths.state_file("claude_usage.json"), Value::Null)
        .get("windows")
        .and_then(|w| serde_json::from_value(w.clone()).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_and_usage() {
        assert_eq!(plan_name("max", "default_claude_max_5x"), "Max 5x");
        assert_eq!(plan_name("pro", ""), "Pro");
        let w = parse_usage(&json!({"seven_day": {"utilization": 12.0}, "five_hour": {"utilization": 40.5, "resets_at": "x"}, "other": 1}));
        assert_eq!(w.iter().map(|x| x.key.as_str()).collect::<Vec<_>>(), ["five_hour", "seven_day"]);
    }
}
