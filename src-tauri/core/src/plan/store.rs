//! The plan in state\plan.json: reminders spoken aloud (say), commands Achaja sends herself
//! (prompt, marked [ZAPLANOWANE]) and background jobs (job). The listener takes what is due;
//! a missed one-off reminder is said late, a missed repeat waits for the next time.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::schedule::{from_ts, next_time, to_ts};
use crate::json::{now, read_json, write_json};
use crate::paths::Paths;

pub const ALLOW: &[&str] = &["mail", "mcp", "delete", "push", "deploy", "home"];
const LATE_REPEAT: f64 = 3600.0;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Item {
    pub id: u64,
    pub kind: String,
    pub text: String,
    pub next: f64,
    #[serde(default)]
    pub repeat: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rules: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Plan {
    #[serde(default)]
    pub items: Vec<Item>,
    #[serde(default = "one")]
    pub next_id: u64,
}

fn one() -> u64 {
    1
}

#[derive(Debug, Clone, PartialEq)]
pub struct Due {
    pub kind: String,
    pub text: String,
    pub item: Item,
}

pub fn kind_name(kind: &str) -> &str {
    match kind {
        "say" => "przypomnienie",
        "prompt" => "polecenie",
        "job" => "zadanie w tle",
        other => other,
    }
}

pub fn load(paths: &Paths) -> Plan {
    let raw: Value = read_json(&paths.state_file("plan.json"), Value::Null);
    serde_json::from_value(raw).unwrap_or(Plan { items: Vec::new(), next_id: 1 })
}

pub fn save(paths: &Paths, plan: &Plan) {
    let _ = write_json(&paths.state_file("plan.json"), plan);
}

pub fn items(paths: &Paths) -> Vec<Item> {
    let mut list = load(paths).items;
    list.sort_by(|a, b| a.next.total_cmp(&b.next));
    list
}

pub fn find(paths: &Paths, id: &str) -> Option<Item> {
    load(paths).items.into_iter().find(|i| i.id.to_string() == id)
}

/// Removes one item or "all"; how many were removed.
pub fn remove(paths: &Paths, target: &str) -> usize {
    let mut plan = load(paths);
    let before = plan.items.len();
    plan.items.retain(|i| target != "all" && i.id.to_string() != target);
    save(paths, &plan);
    before - plan.items.len()
}

/// What is due now among the given kinds; moves repeats on and drops one-offs.
pub fn take_due(paths: &Paths, kinds: &[&str], at: Option<f64>) -> Vec<Due> {
    let t = at.unwrap_or_else(now);
    let mut plan = load(paths);
    let (mut due, mut keep, mut changed) = (Vec::new(), Vec::new(), false);
    for mut item in std::mem::take(&mut plan.items) {
        let stale = (item.kind == "prompt" || item.kind == "job") && item.repeat.is_none() && t - item.next > 6.0 * 3600.0;
        if item.next > t || (!kinds.contains(&item.kind.as_str()) && !(stale && kinds.contains(&"say"))) {
            keep.push(item);
            continue;
        }
        changed = true;
        let late = t - item.next;
        if let Some(rule) = item.repeat.clone() {
            if late <= LATE_REPEAT {
                due.push(Due { kind: item.kind.clone(), text: item.text.clone(), item: item.clone() });
            }
            item.next = next_time(&rule, from_ts(t)).map(to_ts).unwrap_or(t + 86400.0);
            keep.push(item);
        } else {
            let when = from_ts(item.next);
            let (kind, text) = if stale {
                // an old command or job does not start by itself any more; it is only mentioned
                let what = item.name.clone().unwrap_or_else(|| item.text.clone());
                ("say".to_string(), format!("Przegapiłam zaplanowane {} z {}: {what}", kind_name(&item.kind), when.format("%d.%m %H:%M")))
            } else if item.kind == "say" && late > 300.0 {
                ("say".to_string(), format!("Spóźnione przypomnienie z {}: {}", when.format("%H:%M"), item.text))
            } else {
                (item.kind.clone(), item.text.clone())
            };
            due.push(Due { kind, text, item });
        }
    }
    plan.items = keep;
    if changed {
        save(paths, &plan);
    }
    due
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: u64, kind: &str, next: f64, repeat: Option<&str>) -> Item {
        Item { id, kind: kind.into(), text: format!("t{id}"), next, repeat: repeat.map(str::to_string), name: None, rules: None, allow: None }
    }

    #[test]
    fn takes_due_items_by_kind() {
        let dir = tempfile::tempdir().unwrap();
        let p = Paths::at(dir.path(), dir.path());
        let t = 1_800_000_000.0;
        save(&p, &Plan { items: vec![item(1, "say", t - 10.0, None), item(2, "prompt", t - 10.0, None), item(3, "job", t - 10.0, Some("daily 08:00")), item(4, "say", t + 100.0, None)], next_id: 5 });
        let due = take_due(&p, &["say", "job"], Some(t));
        assert_eq!(due.iter().map(|d| d.item.id).collect::<Vec<_>>(), [1, 3]);
        let left: Vec<u64> = load(&p).items.iter().map(|i| i.id).collect();
        assert_eq!(left, [2, 3, 4]);
        assert!(load(&p).items.iter().find(|i| i.id == 3).unwrap().next > t);
    }

    #[test]
    fn stale_commands_are_only_mentioned() {
        let dir = tempfile::tempdir().unwrap();
        let p = Paths::at(dir.path(), dir.path());
        let t = 1_800_000_000.0;
        save(&p, &Plan { items: vec![item(1, "prompt", t - 8.0 * 3600.0, None)], next_id: 2 });
        let due = take_due(&p, &["say"], Some(t));
        assert_eq!(due[0].kind, "say");
        assert!(due[0].text.starts_with("Przegapiłam zaplanowane polecenie"));
    }
}
