//! Project folders (every subfolder of agents.project_roots) and matching a dictated project
//! name: aliases, exact and normalized names, a unique substring, then the closest match.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde_json::Value;

use crate::config::Config;
use crate::error::{AppError, Result};
use crate::text::close_match;

pub fn norm(s: &str) -> String {
    let lower = s.to_lowercase();
    let stripped = ["project-", "project_", "project ", "work-", "work_", "work "]
        .iter()
        .find_map(|p| lower.strip_prefix(p))
        .unwrap_or(&lower)
        .to_string();
    stripped.replace(['-', '_', ' '], "")
}

pub fn all_projects(cfg: &Config) -> BTreeMap<String, PathBuf> {
    let mut out = BTreeMap::new();
    for root in cfg.strs("agents.project_roots") {
        for entry in std::fs::read_dir(&root).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if entry.path().is_dir() && !name.starts_with('.') {
                out.insert(name, entry.path());
            }
        }
    }
    out
}

pub fn aliases(cfg: &Config) -> BTreeMap<String, String> {
    match cfg.get("agents.aliases") {
        Some(Value::Object(map)) => map.iter().filter_map(|(k, v)| v.as_str().map(|v| (k.to_lowercase(), v.to_string()))).collect(),
        _ => BTreeMap::new(),
    }
}

pub fn resolve(query: &str, cfg: &Config) -> Result<(String, PathBuf)> {
    let projects = all_projects(cfg);
    let alias = aliases(cfg);
    let q = query.trim().to_lowercase();
    let found = |name: &str| projects.get(name).map(|p| (name.to_string(), p.clone()));
    if let Some(hit) = alias.get(&q).and_then(|n| found(n)) {
        return Ok(hit);
    }
    if let Some(hit) = found(query) {
        return Ok(hit);
    }
    let by_norm: BTreeMap<String, String> = projects.keys().map(|n| (norm(n), n.clone())).collect();
    let nq = norm(&q);
    if let Some(hit) = by_norm.get(&nq).and_then(|n| found(n)) {
        return Ok(hit);
    }
    let subs: Vec<&String> = by_norm.iter().filter(|(k, _)| !nq.is_empty() && (k.contains(&nq) || nq.contains(k.as_str()))).map(|(_, n)| n).collect();
    if subs.len() == 1 {
        if let Some(hit) = found(subs[0]) {
            return Ok(hit);
        }
    }
    let candidates: Vec<&str> = by_norm.keys().map(String::as_str).chain(alias.keys().map(String::as_str)).collect();
    if let Some(close) = close_match(&nq, candidates, 0.6) {
        let name = by_norm.get(close).or_else(|| alias.get(close));
        if let Some(hit) = name.and_then(|n| found(n)) {
            return Ok(hit);
        }
    }
    Err(AppError::msg(format!("Nie rozpoznano projektu '{query}'. Dostepne: {}", projects.keys().cloned().collect::<Vec<_>>().join(", "))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_dictated_names() {
        let root = tempfile::tempdir().unwrap();
        for d in ["project-vision", "taxi-app", "shop"] {
            std::fs::create_dir(root.path().join(d)).unwrap();
        }
        let cfg = Config::new(serde_json::json!({"agents": {"project_roots": [root.path()], "aliases": {"sklep": "shop"}}}));
        assert_eq!(resolve("vision", &cfg).unwrap().0, "project-vision");
        assert_eq!(resolve("Sklep", &cfg).unwrap().0, "shop");
        assert_eq!(resolve("taxi", &cfg).unwrap().0, "taxi-app");
        assert_eq!(resolve("taksi app", &cfg).unwrap().0, "taxi-app");
        assert!(resolve("zzz", &cfg).is_err());
    }
}
