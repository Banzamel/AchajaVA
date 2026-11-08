//! Home Assistant: the REST client, the guard for locks and gates, `achaja ha` and the areas
//! helper, plus the Settings view's checks (connection test, phones with the HA app).

pub mod areas;
pub mod client;
pub mod commands;
pub mod guard;

use clap::Subcommand;
use serde_json::Value;

use crate::error::{AppError, Result};
use crate::paths::Paths;

#[derive(Debug, Subcommand)]
pub enum HaCmd {
    /// Stan i atrybuty (entity_id, nazwa albo obszar)
    State { target: String },
    /// Klimatyzacja: tryb i temperatura, z odczytem stanu po zmianie
    Climate {
        target: String,
        #[arg(long, value_parser = clap::builder::PossibleValuesParser::new(commands::MODES))]
        mode: Option<String>,
        #[arg(long)]
        temp: Option<f64>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        confirmed: bool,
    },
    /// Dowolna usluga z dozwolonych domen
    Call {
        domain: String,
        service: String,
        #[arg(default_value = "{}")]
        data: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        confirmed: bool,
    },
    /// Obszary: dump (do state/ha_registry.json) albo apply <plan.json>
    Areas { action: String, plan: Option<String> },
}

pub fn run(paths: &Paths, cmd: HaCmd, dry_run: bool, confirmed: bool) -> Result<i32> {
    crate::runtime::block_on(async move {
        match cmd {
            HaCmd::State { target } => commands::state(&mut client::Ha::new(paths, dry_run)?, &target).await,
            HaCmd::Climate { target, mode, temp, dry_run: d, confirmed: c } => {
                let mut ha = client::Ha::new(paths, dry_run || d)?;
                commands::climate(&mut ha, &target, mode.as_deref(), temp, confirmed || c).await
            }
            HaCmd::Call { domain, service, data, dry_run: d, confirmed: c } => {
                let mut ha = client::Ha::new(paths, dry_run || d)?;
                commands::call(&mut ha, &domain, &service, &data, confirmed || c).await
            }
            HaCmd::Areas { action, plan } => match (action.as_str(), plan) {
                ("dump", _) => {
                    let (a, e) = areas::dump(paths).await?;
                    println!("obszary: {a}, encje: {e}");
                    Ok(0)
                }
                ("apply", Some(file)) => {
                    let steps: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(file)?)?;
                    areas::apply(paths, &steps).await?;
                    Ok(0)
                }
                _ => Err(AppError::msg("Uzycie: achaja ha areas dump | apply <plan.json>")),
            },
        }
    })
}

/// Settings window: does HA answer with this address and token (empty token = the saved one).
pub async fn test(paths: &Paths, url: &str, token: &str) -> (bool, String) {
    let token = if token.trim().is_empty() { crate::json::read_text(&paths.ha_token()).unwrap_or_default() } else { token.to_string() };
    let Ok(http) = crate::runtime::http(8) else { return (false, "HTTP".into()) };
    match http.get(format!("{}/api/", url.trim_end_matches('/'))).bearer_auth(token.trim()).send().await {
        Ok(r) => (r.status().is_success(), format!("Home Assistant odpowiada ({})", r.status().as_u16())),
        Err(e) => (false, e.to_string().replace(token.trim(), "***")),
    }
}

/// notify.mobile_app_* services: phones with the Home Assistant app.
pub async fn find_phones(paths: &Paths, url: &str, token: &str) -> Vec<String> {
    let token = if token.trim().is_empty() { crate::json::read_text(&paths.ha_token()).unwrap_or_default() } else { token.to_string() };
    let Ok(http) = crate::runtime::http(8) else { return Vec::new() };
    let Ok(resp) = http.get(format!("{}/api/services", url.trim_end_matches('/'))).bearer_auth(token.trim()).send().await else { return Vec::new() };
    let services: Value = resp.json().await.unwrap_or(Value::Null);
    let mut names: Vec<String> = services
        .as_array()
        .into_iter()
        .flatten()
        .filter(|d| d["domain"] == "notify")
        .flat_map(|d| d["services"].as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()).unwrap_or_default())
        .filter(|n| n.starts_with("mobile_app_"))
        .collect();
    names.sort();
    names
}
