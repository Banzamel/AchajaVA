//! Token use of the Achaja session, background jobs and project agents: counted per voice
//! command (from the session's transcript) and per job / agent run (from `claude -p`'s result),
//! kept in state\tokens.jsonl, priced at API rates (an estimate: Achaja runs on a subscription),
//! summed into statistics, and used by the limits (a spoken warning near the plan's limit,
//! recurring jobs paused).

pub mod guard;
pub mod ledger;
pub mod price;
pub mod spoken;
pub mod stats;
pub mod transcript;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Token counts of one or more model calls. Cache writes are split by lifetime because the
/// one-hour cache costs more to write than the five-minute one.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Tokens {
    #[serde(default)]
    pub input: u64,
    #[serde(default)]
    pub output: u64,
    #[serde(default)]
    pub cache_read: u64,
    #[serde(default)]
    pub cache_5m: u64,
    #[serde(default)]
    pub cache_1h: u64,
}

fn n(v: &Value, key: &str) -> u64 {
    v.get(key).and_then(Value::as_u64).unwrap_or(0)
}

impl Tokens {
    pub fn add(&mut self, o: &Tokens) {
        self.input += o.input;
        self.output += o.output;
        self.cache_read += o.cache_read;
        self.cache_5m += o.cache_5m;
        self.cache_1h += o.cache_1h;
    }

    pub fn total(&self) -> u64 {
        self.input + self.output + self.cache_read + self.cache_5m + self.cache_1h
    }

    /// The API's `usage` object (transcript lines, the `usage` of a `claude -p` result).
    pub fn from_usage(u: &Value) -> Tokens {
        let split = &u["cache_creation"];
        let (m5, h1) = (n(split, "ephemeral_5m_input_tokens"), n(split, "ephemeral_1h_input_tokens"));
        // without the split every cache write counts as the cheaper five-minute one
        let rest = n(u, "cache_creation_input_tokens").saturating_sub(m5 + h1);
        Tokens { input: n(u, "input_tokens"), output: n(u, "output_tokens"), cache_read: n(u, "cache_read_input_tokens"), cache_5m: m5 + rest, cache_1h: h1 }
    }

    /// One model's entry of `modelUsage` in a `claude -p` result (camelCase).
    pub fn from_model_usage(u: &Value) -> Tokens {
        let (m5, h1) = (n(u, "cacheCreationEphemeral5mInputTokens"), n(u, "cacheCreationEphemeral1hInputTokens"));
        let rest = n(u, "cacheCreationInputTokens").saturating_sub(m5 + h1);
        Tokens { input: n(u, "inputTokens"), output: n(u, "outputTokens"), cache_read: n(u, "cacheReadInputTokens"), cache_5m: m5 + rest, cache_1h: h1 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_both_usage_shapes() {
        let api = json!({"input_tokens": 2, "output_tokens": 200, "cache_read_input_tokens": 30, "cache_creation_input_tokens": 50,
            "cache_creation": {"ephemeral_1h_input_tokens": 40, "ephemeral_5m_input_tokens": 0}});
        assert_eq!(Tokens::from_usage(&api), Tokens { input: 2, output: 200, cache_read: 30, cache_5m: 10, cache_1h: 40 });
        let model = json!({"inputTokens": 1, "outputTokens": 4, "cacheReadInputTokens": 5, "cacheCreationInputTokens": 7, "cacheCreationEphemeral1hInputTokens": 7});
        let mut t = Tokens::from_model_usage(&model);
        assert_eq!(t, Tokens { input: 1, output: 4, cache_read: 5, cache_5m: 0, cache_1h: 7 });
        t.add(&Tokens { input: 1, ..Default::default() });
        assert_eq!(t.total(), 18);
    }
}
