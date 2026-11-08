//! PreToolUse hook for the home-assistant MCP server: one command for the area "garage" or the
//! domain "cover" could open a gate, so the target is expanded to entities (over the REST API)
//! and refused when it reaches a lock, alarm, gate or garage, or when it cannot be checked.
//! After the user's yes, Achaja does it with `achaja ha ... --confirmed` (MCP has no such flag).

use serde_json::Value;

use super::gate::deny;
use crate::homeassistant::client::{eid, friendly, Ha};
use crate::homeassistant::guard::{as_list, guarded, GUARDED, GUARDED_CLASSES, GUARDED_WORDS};
use crate::paths::Paths;
use crate::text::fold;

const READ_ONLY: &[&str] = &["Get", "get_", "list_"];
const RISKY_PREFIXES: &[&str] = &["HassTurnOn", "HassTurnOff", "HassToggle", "HassSetPosition"];
const RISKY_WORDS: &[&str] = &["lock", "alarm", "cover", "valve", "open", "close", "gate", "garage", "door"];

pub fn deny_home(reason: &str) -> Value {
    deny(format!(
        "WYMAGA POTWIERDZENIA: {reason}. Zamki, alarmy, bramy i garaze steruje sie tylko po 'tak' uzytkownika: zapytaj glosowo, a po 'tak' wykonaj to przez achaja.cmd ha z --confirmed (np. achaja.cmd ha call cover open_cover '{{\"entity_id\": \"cover.brama\"}}' --confirmed). Jesli to zwykle urzadzenie (nie zamek, brama ani alarm), wskaz je dokladna nazwa albo uzyj achaja.cmd ha call z dokladnym entity_id - wtedy bez pytania."
    ))
}

/// The entities the call may reach; None = cannot tell.
async fn entities_for(ha: &mut Ha, args: &Value) -> Option<Vec<Value>> {
    let domains: Vec<String> = as_list(&args["domain"]).iter().map(|d| d.to_lowercase()).collect();
    let states = ha.states().await.ok()?;
    let mut pool: Option<Vec<String>> = None;
    for area in as_list(&args["area"]) {
        let ids = ha.area_entities(&area).await.ok()?;
        if ids.is_empty() {
            return None; // an area we do not know: no guessing
        }
        pool = Some(match pool {
            None => ids,
            Some(p) => p.into_iter().filter(|i| ids.contains(i)).collect(),
        });
    }
    for floor in as_list(&args["floor"]) {
        let q = serde_json::to_string(&floor).ok()?;
        let tpl = format!("{{{{ ((floor_areas({q}) | map('area_entities') | sum(start=[])) + (floor_areas({q}) | map('area_devices') | sum(start=[]) | map('device_entities') | sum(start=[]))) | unique | list | tojson }}}}");
        let ids = ha.template_ids(&tpl).await.ok()?;
        if ids.is_empty() {
            return None;
        }
        pool = Some(match pool {
            None => ids,
            Some(p) => p.into_iter().filter(|i| ids.contains(i)).collect(),
        });
    }
    let in_domain = |id: &str| domains.is_empty() || domains.iter().any(|d| id.split('.').next() == Some(d.as_str()));
    let mut candidates: Vec<Value> = states.into_iter().filter(|s| in_domain(&eid(s)) && pool.as_ref().map_or(true, |p| p.contains(&eid(s)))).collect();
    let names = as_list(&args["name"]);
    if !names.is_empty() {
        let wanted: Vec<String> = names.iter().map(|n| fold(n)).collect();
        candidates.retain(|s| wanted.iter().any(|w| fold(&friendly(s)).contains(w) || fold(&eid(s)).contains(w)));
        if candidates.is_empty() {
            return None; // a name outside the entity list (an alias): unknown what HA picks
        }
    }
    Some(candidates)
}

async fn check(paths: &Paths, tool: &str, args: &Value) -> Option<String> {
    let text = fold(&[as_list(&args["name"]), as_list(&args["area"]), as_list(&args["floor"])].concat().join(" "));
    if GUARDED_WORDS.iter().any(|w| text.contains(w)) {
        return Some(format!("{tool} dla '{text}'"));
    }
    let domains = as_list(&args["domain"]);
    if domains.iter().any(|d| GUARDED.contains(&d.to_lowercase().as_str())) {
        return Some(format!("{tool} dla domeny {}", domains.join(", ")));
    }
    let classes = as_list(&args["device_class"]);
    if classes.iter().any(|c| GUARDED_CLASSES.contains(&c.to_lowercase().as_str())) {
        return Some(format!("{tool} dla urzadzen typu {}", classes.join(", ")));
    }
    let found = match Ha::new(paths, false) {
        Ok(mut ha) => entities_for(&mut ha, args).await,
        Err(_) => None,
    };
    let Some(found) = found else {
        return Some(format!("{tool}: nie udalo sie sprawdzic, ktore urzadzenia obejmie to polecenie"));
    };
    found.iter().find(|s| guarded(s)).map(|s| format!("{tool} obejmuje {} ({})", eid(s), friendly(s)))
}

pub fn decide(paths: &Paths, data: &Value) -> Option<Value> {
    let tool = data["tool_name"].as_str().unwrap_or("").rsplit("__").next().unwrap_or("").to_string();
    let args = if data["tool_input"].is_object() { data["tool_input"].clone() } else { return None };
    if tool.is_empty() || READ_ONLY.iter().any(|p| tool.starts_with(p)) {
        return None;
    }
    if GUARDED_WORDS.iter().any(|w| fold(&tool).contains(w)) {
        return Some(deny_home(&format!("skrypt {tool}"))); // an HA script exposed as a tool
    }
    if !(RISKY_PREFIXES.iter().any(|p| tool.starts_with(p)) || RISKY_WORDS.iter().any(|w| tool.to_lowercase().contains(w))) {
        return None;
    }
    let reason = crate::runtime::block_on(async { Ok(check(paths, &tool, &args).await) }).unwrap_or_else(|e| Some(format!("blad sprawdzania ({e})")));
    reason.map(|r| deny_home(&r))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn refuses_gate_words_without_asking_ha() {
        let dir = tempfile::tempdir().unwrap();
        let p = Paths::at(dir.path(), dir.path());
        let out = decide(&p, &json!({"tool_name": "mcp__home-assistant__HassTurnOn", "tool_input": {"name": "Brama"}}));
        assert!(out.is_some());
        assert!(decide(&p, &json!({"tool_name": "mcp__home-assistant__GetLiveContext", "tool_input": {}})).is_none());
        assert!(decide(&p, &json!({"tool_name": "mcp__home-assistant__HassLightSet", "tool_input": {"name": "x"}})).is_none());
        // no address or token: cannot check, so it is refused
        assert!(decide(&p, &json!({"tool_name": "mcp__home-assistant__HassTurnOff", "tool_input": {"area": "salon"}})).is_some());
    }
}
