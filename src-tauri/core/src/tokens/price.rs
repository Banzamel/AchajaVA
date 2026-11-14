//! API prices per million tokens, used to estimate what the work would cost on the API. Built-in
//! defaults (the price list of November 2026) can be overridden or extended in config.json:
//! "pricing": {"claude-opus-5-5": {"input": 4, "output": 20, "cache_read": 0.2}}. A model is
//! matched by the longest key its id contains, so "opus" also covers future Opus versions.

use serde_json::Value;

use super::Tokens;
use crate::config::Config;

#[derive(Debug, Clone, PartialEq)]
pub struct Price {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_5m: f64,
    pub cache_1h: f64,
}

impl Price {
    /// Cache writes cost 1.25x (five minutes) and 2x (one hour) the input price.
    fn new(input: f64, output: f64, cache_read: f64) -> Price {
        Price { input, output, cache_read, cache_5m: input * 1.25, cache_1h: input * 2.0 }
    }
}

const DEFAULTS: &[(&str, f64, f64, f64)] = &[
    ("claude-fable-5-2", 8.0, 40.0, 0.2),
    ("claude-opus-5-5", 4.0, 20.0, 0.2),
    ("claude-sonnet-5-5", 2.0, 10.0, 0.1),
    ("claude-haiku-5-5", 0.1, 0.5, 0.01),
    ("fable", 10.0, 50.0, 1.0),
    ("mythos", 10.0, 50.0, 1.0),
    ("opus", 5.0, 25.0, 0.5),
    ("sonnet", 2.0, 10.0, 0.2),
    ("haiku", 1.0, 5.0, 0.1),
];

pub struct Prices(Vec<(String, Price)>);

impl Prices {
    pub fn defaults() -> Prices {
        Prices(DEFAULTS.iter().map(|(k, i, o, c)| (k.to_string(), Price::new(*i, *o, *c))).collect())
    }

    pub fn from_config(cfg: &Config) -> Prices {
        let mut p = Prices::defaults();
        if let Value::Object(map) = cfg.section("pricing") {
            for (key, v) in map.iter().filter(|(k, v)| !k.starts_with('_') && v.is_object()) {
                let f = |name: &str| v.get(name).and_then(Value::as_f64);
                let base = p.find(key).cloned();
                let input = f("input").or(base.as_ref().map(|b| b.input)).unwrap_or(0.0);
                let output = f("output").or(base.as_ref().map(|b| b.output)).unwrap_or(0.0);
                let read = f("cache_read").or(base.as_ref().filter(|_| f("input").is_none()).map(|b| b.cache_read)).unwrap_or(input * 0.1);
                let mut price = Price::new(input, output, read);
                price.cache_5m = f("cache_write_5m").unwrap_or(price.cache_5m);
                price.cache_1h = f("cache_write_1h").unwrap_or(price.cache_1h);
                p.0.retain(|(k, _)| k != key);
                p.0.push((key.clone(), price));
            }
        }
        p
    }

    pub fn find(&self, model: &str) -> Option<&Price> {
        let m = model.to_lowercase();
        self.0.iter().filter(|(k, _)| m.contains(&k.to_lowercase())).max_by_key(|(k, _)| k.len()).map(|(_, p)| p)
    }

    /// USD; 0 for a model without a price.
    pub fn cost(&self, model: &str, t: &Tokens) -> f64 {
        let Some(p) = self.find(model) else { return 0.0 };
        let f = |count: u64, per: f64| count as f64 * per / 1_000_000.0;
        f(t.input, p.input) + f(t.output, p.output) + f(t.cache_read, p.cache_read) + f(t.cache_5m, p.cache_5m) + f(t.cache_1h, p.cache_1h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn prices_by_longest_match_and_config() {
        let p = Prices::defaults();
        let t = Tokens { input: 1_000_000, output: 1_000_000, cache_read: 1_000_000, cache_5m: 0, cache_1h: 1_000_000 };
        assert!((p.cost("claude-opus-5-5", &t) - (4.0 + 20.0 + 0.2 + 8.0)).abs() < 1e-9);
        assert!((p.cost("claude-opus-6", &Tokens { output: 1_000_000, ..Default::default() }) - 25.0).abs() < 1e-9);
        assert_eq!(p.cost("gpt-x", &t), 0.0);
        let cfg = Config::new(json!({"pricing": {"_comment": "x", "claude-opus-5-5": {"output": 30}, "local": {"input": 1}}}));
        let p = Prices::from_config(&cfg);
        assert_eq!(p.find("claude-opus-5-5").map(|x| (x.input, x.output)), Some((4.0, 30.0)));
        assert_eq!(p.find("my-local-model").map(|x| x.cache_read), Some(0.1));
    }
}
