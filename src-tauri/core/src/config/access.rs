//! Typed reads of the JSON configuration by dotted path ("wake.end_match"), with defaults —
//! config.json is edited by hand, so every value may be missing or of another type.

use serde_json::Value;

#[derive(Debug, Clone)]
pub struct Config {
    pub raw: Value,
}

impl Config {
    pub fn new(raw: Value) -> Self {
        Self { raw }
    }

    pub fn get(&self, path: &str) -> Option<&Value> {
        let mut cur = &self.raw;
        for part in path.split('.') {
            cur = cur.get(part)?;
        }
        Some(cur)
    }

    pub fn str_or(&self, path: &str, default: &str) -> String {
        match self.get(path) {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Number(n)) => n.to_string(),
            _ => default.to_string(),
        }
    }

    pub fn f64_or(&self, path: &str, default: f64) -> f64 {
        match self.get(path) {
            Some(Value::Number(n)) => n.as_f64().unwrap_or(default),
            Some(Value::String(s)) => s.trim().parse().unwrap_or(default),
            _ => default,
        }
    }

    pub fn bool_or(&self, path: &str, default: bool) -> bool {
        match self.get(path) {
            Some(Value::Bool(b)) => *b,
            Some(Value::Number(n)) => n.as_f64().map(|v| v != 0.0).unwrap_or(default),
            Some(Value::Null) | None => default,
            Some(_) => default,
        }
    }

    /// A list of strings; a single string counts as a one-element list.
    pub fn strs(&self, path: &str) -> Vec<String> {
        match self.get(path) {
            Some(Value::Array(items)) => {
                items.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()
            }
            Some(Value::String(s)) if !s.is_empty() => vec![s.clone()],
            _ => Vec::new(),
        }
    }

    /// The first word of a list (e.g. the first end word), or the default.
    pub fn first(&self, path: &str, default: &str) -> String {
        self.strs(path).into_iter().next().unwrap_or_else(|| default.to_string())
    }

    pub fn language(&self) -> String {
        self.str_or("language", "pl")
    }

    /// "pl" or "en" for texts that exist in those two languages.
    pub fn lang2(&self) -> &'static str {
        if self.language() == "en" {
            "en"
        } else {
            "pl"
        }
    }

    pub fn section(&self, path: &str) -> Value {
        self.get(path).cloned().unwrap_or(Value::Null)
    }
}
