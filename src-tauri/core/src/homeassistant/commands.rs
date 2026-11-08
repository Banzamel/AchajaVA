//! `achaja ha state | climate | call`: what the MCP server lacks (the air conditioner's HVAC
//! mode above all). Exit codes: 0 ok, 1 error, 2 the state after the change differs, 3 needs
//! --confirmed.

use serde_json::{json, Value};

use super::client::{eid, friendly, Ha};
use super::guard::{guarded, unconfirmed_risk, ALLOWED};
use crate::error::{AppError, Result};

pub const MODES: &[&str] = &["heat", "cool", "auto", "heat_cool", "dry", "fan_only", "off"];
const SETTLE_MS: u64 = 2500;

fn describe(s: &Value, full: bool) -> String {
    let line = format!("{} ({}): {}", eid(s), friendly(s), s["state"].as_str().unwrap_or(""));
    if !full {
        return line;
    }
    let skip = ["friendly_name", "entity_picture", "icon", "supported_features", "supported_color_modes"];
    let attrs: Vec<String> = s["attributes"]
        .as_object()
        .map(|o| o.iter().filter(|(k, _)| !skip.contains(&k.as_str())).map(|(k, v)| format!("  {k}: {v}")).collect())
        .unwrap_or_default();
    if attrs.is_empty() {
        line
    } else {
        format!("{line}\n{}", attrs.join("\n"))
    }
}

pub fn refuse(what: &str) -> i32 {
    println!("WYMAGA POTWIERDZENIA: {what} - zamki, alarmy, bramy i garaze steruje tylko po 'tak' uzytkownika (uruchom ponownie z --confirmed).");
    3
}

pub async fn state(ha: &mut Ha, target: &str) -> Result<i32> {
    let found = ha.find(target, None).await?;
    if found.is_empty() {
        return Err(AppError::msg(format!("Nie znalazlam '{target}' (ani urzadzenia, ani obszaru).")));
    }
    for s in found.iter().take(40) {
        println!("{}", describe(s, found.len() == 1));
    }
    if found.len() > 40 {
        println!("... i {} wiecej", found.len() - 40);
    }
    Ok(0)
}

pub async fn climate(ha: &mut Ha, target: &str, mode: Option<&str>, temp: Option<f64>, confirmed: bool) -> Result<i32> {
    let found = ha.find(target, Some("climate")).await?;
    let dev = match found.len() {
        0 => return Err(AppError::msg(format!("Nie znalazlam urzadzenia '{target}' (climate)."))),
        1 => found[0].clone(),
        _ => {
            let names: Vec<String> = found.iter().take(8).map(|s| format!("{} ({})", eid(s), friendly(s))).collect();
            return Err(AppError::msg(format!("Kilka urzadzen pasuje do '{target}': {}. Podaj dokladniejsza nazwe albo entity_id.", names.join(", "))));
        }
    };
    let id = eid(&dev);
    if guarded(&dev) && !confirmed {
        return Ok(refuse(&id));
    }
    let modes: Vec<String> = dev["attributes"]["hvac_modes"].as_array().map(|a| a.iter().filter_map(|m| m.as_str().map(str::to_string)).collect()).unwrap_or_default();
    if let Some(m) = mode {
        if !modes.iter().any(|x| x == m) {
            let list = if modes.is_empty() { "brak informacji".into() } else { modes.join(", ") };
            return Err(AppError::msg(format!("{} nie obsluguje trybu '{m}'. Dostepne tryby: {list}.", friendly(&dev))));
        }
    }
    if let Some(t) = temp {
        let (lo, hi) = (dev["attributes"]["min_temp"].as_f64(), dev["attributes"]["max_temp"].as_f64());
        if let (Some(lo), Some(hi)) = (lo, hi) {
            if !(lo..=hi).contains(&t) {
                return Err(AppError::msg(format!("Temperatura {t} poza zakresem urzadzenia ({lo}-{hi}).")));
            }
        }
        if mode == Some("off") {
            return Err(AppError::msg("Nie ustawiam temperatury przy wylaczaniu (--mode off)."));
        }
    }
    if mode.is_none() && temp.is_none() {
        return Err(AppError::msg("Podaj --mode albo --temp."));
    }
    // the mode switches the device on; the temperature after it (some units reset it on a mode change)
    if let Some(m) = mode {
        ha.service("climate", "set_hvac_mode", &json!({"entity_id": id, "hvac_mode": m})).await?;
    } else if dev["state"] == "off" {
        ha.service("climate", "turn_on", &json!({"entity_id": id})).await?;
    }
    if let Some(t) = temp {
        let value = if t.fract() == 0.0 { json!(t as i64) } else { json!(t) };
        ha.service("climate", "set_temperature", &json!({"entity_id": id, "temperature": value})).await?;
    }
    if ha.dry_run {
        println!("[dry-run] potem GET /api/states/{id} po 2.5 s i porownanie z zadanym stanem");
        return Ok(0);
    }
    tokio::time::sleep(std::time::Duration::from_millis(SETTLE_MS)).await;
    let now = ha.state(&id).await?;
    let a = &now["attributes"];
    let st = now["state"].as_str().unwrap_or("");
    println!("{}: tryb {st}, temperatura zadana {}, w pomieszczeniu {}", friendly(&now), a["temperature"], a["current_temperature"]);
    let mut problems = Vec::new();
    if let Some(m) = mode.filter(|m| *m != st) {
        problems.push(format!("tryb jest '{st}' zamiast '{m}'"));
    }
    if let (Some(t), Some(set)) = (temp, a["temperature"].as_f64()) {
        if set != t {
            problems.push(format!("temperatura zadana to {set} zamiast {t}"));
        }
    }
    if !problems.is_empty() {
        println!("UWAGA: {}.", problems.join("; "));
        return Ok(2);
    }
    println!("OK: stan zgodny z zadanym.");
    Ok(0)
}

pub async fn call(ha: &mut Ha, domain: &str, service: &str, data: &str, confirmed: bool) -> Result<i32> {
    if !ALLOWED.contains(&domain) {
        return Err(AppError::msg(format!("Domena '{domain}' nie jest dozwolona. Dozwolone: {}.", ALLOWED.join(", "))));
    }
    let data: Value = serde_json::from_str(if data.trim().is_empty() { "{}" } else { data }).map_err(|e| AppError::msg(format!("Niepoprawny JSON: {e}")))?;
    if !data.is_object() {
        return Err(AppError::msg("Dane uslugi musza byc obiektem JSON, np. {\"entity_id\": \"light.salon\"}."));
    }
    if !confirmed {
        if let Some(why) = unconfirmed_risk(ha, domain, service, &data).await {
            return Ok(refuse(&why));
        }
    }
    let result = ha.service(domain, service, &data).await?;
    if ha.dry_run {
        return Ok(0);
    }
    let changed: Vec<String> = result.as_array().map(|a| a.iter().take(20).map(|s| format!("{}: {}", eid(s), s["state"].as_str().unwrap_or(""))).collect()).unwrap_or_default();
    if changed.is_empty() {
        println!("OK (usluga wykonana).");
    } else {
        println!("OK: {}", changed.join(", "));
    }
    Ok(0)
}
