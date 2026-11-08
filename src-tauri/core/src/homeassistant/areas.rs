//! Areas in Home Assistant over its WebSocket API (`achaja ha areas dump | apply plan.json`):
//! listing rooms and entities without one, and assigning devices or entities to areas — Assist
//! then understands "turn off the lights in the bedroom" much better.

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio_tungstenite::tungstenite::Message;

use crate::config;
use crate::error::{AppError, Result};
use crate::json::{read_text, write_json};
use crate::paths::Paths;

type Ws = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

struct Conn {
    ws: Ws,
    n: u64,
}

fn err(e: impl std::fmt::Display) -> AppError {
    AppError::Http(format!("Home Assistant: {e}"))
}

async fn recv(ws: &mut Ws) -> Result<Value> {
    while let Some(msg) = ws.next().await {
        if let Message::Text(t) = msg.map_err(err)? {
            return Ok(serde_json::from_str(&t)?);
        }
    }
    Err(err("connection closed"))
}

impl Conn {
    async fn open(paths: &Paths) -> Result<Conn> {
        crate::runtime::install_tls();
        let url = config::load_or_default(paths).str_or("home_assistant.url", "");
        if url.is_empty() {
            return Err(AppError::msg("Brak home_assistant.url w config.json"));
        }
        let ws_url = format!("{}/api/websocket", url.replace("https://", "wss://").replace("http://", "ws://").trim_end_matches('/'));
        let (mut ws, _) = tokio_tungstenite::connect_async(ws_url.as_str()).await.map_err(err)?;
        recv(&mut ws).await?; // auth_required
        let token = read_text(&paths.ha_token()).unwrap_or_default();
        ws.send(Message::Text(json!({"type": "auth", "access_token": token.trim()}).to_string().into())).await.map_err(err)?;
        if recv(&mut ws).await?["type"] != "auth_ok" {
            return Err(AppError::msg("Autoryzacja HA nieudana"));
        }
        Ok(Conn { ws, n: 0 })
    }

    async fn call(&mut self, kind: &str, extra: Value) -> Result<Value> {
        self.n += 1;
        let mut msg = json!({"id": self.n, "type": kind});
        if let (Some(m), Some(e)) = (msg.as_object_mut(), extra.as_object()) {
            m.extend(e.clone());
        }
        self.ws.send(Message::Text(msg.to_string().into())).await.map_err(err)?;
        loop {
            let reply = recv(&mut self.ws).await?;
            if reply["id"] == json!(self.n) {
                if reply["success"] == json!(false) {
                    return Err(AppError::msg(format!("{kind}: {}", reply["error"])));
                }
                return Ok(reply["result"].clone());
            }
        }
    }
}

pub async fn dump(paths: &Paths) -> Result<(usize, usize)> {
    let mut c = Conn::open(paths).await?;
    let areas = c.call("config/area_registry/list", json!({})).await?;
    let devices = c.call("config/device_registry/list", json!({})).await?;
    let entities = c.call("config/entity_registry/list", json!({})).await?;
    let states = c.call("get_states", json!({})).await?;
    let list = |v: &Value| v.as_array().cloned().unwrap_or_default();
    let area_name = |id: &Value| list(&areas).iter().find(|a| a["area_id"] == *id).map(|a| a["name"].clone()).unwrap_or(Value::Null);
    let mut rows = Vec::new();
    for e in list(&entities) {
        if !e["disabled_by"].is_null() || !e["hidden_by"].is_null() || !e["entity_category"].is_null() {
            continue;
        }
        let dev = list(&devices).into_iter().find(|d| d["id"] == e["device_id"]).unwrap_or(Value::Null);
        let area = if e["area_id"].is_null() { dev["area_id"].clone() } else { e["area_id"].clone() };
        let st = list(&states).into_iter().find(|s| s["entity_id"] == e["entity_id"]).unwrap_or(Value::Null);
        let name = [&st["attributes"]["friendly_name"], &e["name"], &e["original_name"]].into_iter().find(|v| !v.is_null()).cloned().unwrap_or(Value::Null);
        let device = if dev.is_null() { Value::Null } else if !dev["name_by_user"].is_null() { dev["name_by_user"].clone() } else { dev["name"].clone() };
        let from = if !e["area_id"].is_null() { json!("entity") } else if !dev["area_id"].is_null() { json!("device") } else { Value::Null };
        rows.push(json!({"entity_id": e["entity_id"], "name": name, "device_id": e["device_id"], "device": device, "area": area_name(&area), "area_from": from}));
    }
    let out_areas: Vec<Value> = list(&areas).iter().map(|a| json!({"area_id": a["area_id"], "name": a["name"]})).collect();
    let counts = (out_areas.len(), rows.len());
    write_json(&paths.state_file("ha_registry.json"), &json!({"areas": out_areas, "entities": rows}))?;
    Ok(counts)
}

/// plan: [{"kind": "device"|"entity", "id", "area_id", "area_name"?}] — an unknown area is created.
pub async fn apply(paths: &Paths, plan: &[Value]) -> Result<()> {
    let mut c = Conn::open(paths).await?;
    let mut existing: Vec<String> = c.call("config/area_registry/list", json!({})).await?.as_array().map(|a| a.iter().filter_map(|x| x["area_id"].as_str().map(str::to_string)).collect()).unwrap_or_default();
    for step in plan {
        let mut area = step["area_id"].as_str().unwrap_or("").to_string();
        if !existing.contains(&area) {
            let created = c.call("config/area_registry/create", json!({"name": step["area_name"]})).await?;
            area = created["area_id"].as_str().unwrap_or("").to_string();
            existing.push(area.clone());
            println!("Utworzono obszar {} ({area})", step["area_name"]);
        }
        let id = step["id"].clone();
        if step["kind"] == "device" {
            c.call("config/device_registry/update", json!({"device_id": id, "area_id": area})).await?;
        } else {
            c.call("config/entity_registry/update", json!({"entity_id": id, "area_id": area})).await?;
        }
        println!("{} {} -> {area}", step["kind"].as_str().unwrap_or(""), step.get("label").unwrap_or(&id));
    }
    Ok(())
}
