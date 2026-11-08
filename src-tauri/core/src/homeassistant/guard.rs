//! Locks, alarms, gates and garages only after the user's spoken yes. A call aimed at an area,
//! floor, device or label is expanded to the entities it reaches; when that cannot be checked,
//! it counts as guarded ("when in doubt, ask").

use serde_json::Value;

use super::client::{eid, Ha};
use crate::text::fold;

pub const ALLOWED: &[&str] = &[
    "climate", "light", "switch", "cover", "fan", "media_player", "vacuum", "script", "lock", "alarm_control_panel",
];
pub const GUARDED: &[&str] = &["lock", "alarm_control_panel"];
pub const GUARDED_CLASSES: &[&str] = &["garage", "gate", "door"];
pub const GUARDED_WORDS: &[&str] = &[
    "bram", "furtk", "garaz", "gate", "garage", "zamek", "zamk", "lock", "alarm", "drzwi", "wjazd", "szlaban",
];
const TARGET_KEYS: &[&str] = &["area_id", "device_id", "floor_id", "label_id"];
const SCRIPT_SERVICES: &[&str] = &["turn_on", "turn_off", "toggle", "reload"];

/// A lock, alarm, gate or garage.
pub fn guarded(entity: &Value) -> bool {
    let id = eid(entity);
    let attrs = &entity["attributes"];
    let domain = id.split('.').next().unwrap_or("");
    if GUARDED.contains(&domain) || attrs["device_class"].as_str().is_some_and(|c| GUARDED_CLASSES.contains(&c)) {
        return true;
    }
    let text = fold(&format!("{id} {}", attrs["friendly_name"].as_str().unwrap_or("")));
    GUARDED_WORDS.iter().any(|w| text.contains(w))
}

/// entity_id/area_id/... may be a string (also "a, b"), a list or missing.
pub fn as_list(value: &Value) -> Vec<String> {
    let items: Vec<String> = match value {
        Value::Null => return Vec::new(),
        Value::Array(a) => a.iter().map(|v| v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string())).collect(),
        Value::String(s) => vec![s.clone()],
        other => vec![other.to_string()],
    };
    items.iter().flat_map(|v| v.split(',').map(|p| p.trim().to_string())).filter(|p| !p.is_empty()).collect()
}

/// Every place HA takes a target from (top level and "target"): key -> values.
pub fn call_targets(data: &Value) -> Vec<(String, Vec<String>)> {
    let mut keys = vec!["entity_id"];
    keys.extend_from_slice(TARGET_KEYS);
    keys.iter()
        .map(|k| {
            let mut vals = as_list(&data[*k]);
            vals.extend(as_list(&data["target"][*k]));
            (k.to_string(), vals)
        })
        .collect()
}

fn quote(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "''".into())
}

/// The entities a call with area/device/floor/label targets reaches; None when unknown.
async fn expand(ha: &Ha, targets: &[(String, Vec<String>)]) -> Option<Vec<String>> {
    let mut pieces = Vec::new();
    for (key, values) in targets {
        for v in values {
            let q = quote(v);
            match key.as_str() {
                "area_id" => pieces.push(format!("area_entities({q}) + (area_devices({q}) | map('device_entities') | sum(start=[]))")),
                "device_id" => pieces.push(format!("device_entities({q})")),
                "floor_id" => pieces.push(format!(
                    "(floor_areas({q}) | map('area_entities') | sum(start=[])) + (floor_areas({q}) | map('area_devices') | sum(start=[]) | map('device_entities') | sum(start=[]))"
                )),
                "label_id" => pieces.push(format!(
                    "label_entities({q}) + (label_devices({q}) | map('device_entities') | sum(start=[])) + (label_areas({q}) | map('area_entities') | sum(start=[]))"
                )),
                _ => {}
            }
        }
    }
    if pieces.is_empty() {
        return Some(Vec::new());
    }
    ha.template_ids(&format!("{{{{ ({}) | unique | list | tojson }}}}", pieces.join(" + "))).await.ok()
}

/// Why the call needs --confirmed, or None.
pub async fn unconfirmed_risk(ha: &mut Ha, domain: &str, service: &str, data: &Value) -> Option<String> {
    if GUARDED.contains(&domain) {
        return Some(domain.to_string());
    }
    let targets = call_targets(data);
    let mut ids: Vec<String> = targets.iter().find(|(k, _)| k == "entity_id").map(|(_, v)| v.clone()).unwrap_or_default();
    if ids.iter().any(|i| i.eq_ignore_ascii_case("all")) {
        return Some("entity_id 'all' (obejmuje wszystkie urzadzenia, takze bramy i zamki)".into());
    }
    if domain == "script" && !SCRIPT_SERVICES.contains(&service) {
        ids.push(format!("script.{service}")); // script.<name> runs a script without entity_id
    }
    let wide: Vec<(String, Vec<String>)> = targets.into_iter().filter(|(k, v)| k != "entity_id" && !v.is_empty()).collect();
    if !wide.is_empty() {
        match expand(ha, &wide).await {
            Some(found) => ids.extend(found),
            None => return Some("obszar/urzadzenie, ktorego nie dalo sie sprawdzic (moze obejmowac brame lub zamek)".into()),
        }
    } else if ids.is_empty() && domain == "cover" {
        return Some("rolety bez wskazanego celu (moze obejmowac brame lub garaz)".into());
    }
    let states = ha.states().await.unwrap_or_default();
    for id in ids {
        let entity = states.iter().find(|s| eid(s) == id).cloned().unwrap_or_else(|| serde_json::json!({ "entity_id": id }));
        if guarded(&entity) {
            return Some(id);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn guards_locks_gates_and_names() {
        assert!(guarded(&json!({"entity_id": "lock.front", "attributes": {}})));
        assert!(guarded(&json!({"entity_id": "cover.x", "attributes": {"device_class": "garage"}})));
        assert!(guarded(&json!({"entity_id": "switch.y", "attributes": {"friendly_name": "Brama wjazdowa"}})));
        assert!(!guarded(&json!({"entity_id": "light.desk", "attributes": {"friendly_name": "Biurko"}})));
    }

    #[test]
    fn collects_targets_from_both_places() {
        let t = call_targets(&json!({"entity_id": "light.a, light.b", "target": {"area_id": ["salon"]}}));
        assert_eq!(t[0].1, ["light.a", "light.b"]);
        assert_eq!(t[1], ("area_id".to_string(), vec!["salon".to_string()]));
    }
}
