//! Recognizing the control words in Vosk's results: the wake word (its own utterance, every
//! word confident enough) and the end word (fuzzy: the small model hears "wykonaj" as
//! "wykonanie", "wygodniej"...).

use std::collections::HashSet;

use serde_json::Value;

use crate::text::ratio;

/// The wake phrase in a finished utterance, its words at least `min_conf` sure. Variants of one
/// phrase ("aha ja", "a ha ja", "acha ja") share the confidence of the words they differ in, so
/// for those words the threshold is divided by the number of variants; words common to all of
/// them ("ja") need the full threshold. `isolated`: the utterance is the phrase alone.
pub fn wake_heard(result: &Value, phrases: &[String], min_conf: f64, isolated: bool) -> bool {
    let text = result["text"].as_str().unwrap_or("");
    let words = result["result"].as_array().cloned().unwrap_or_default();
    let sets: Vec<HashSet<&str>> = phrases.iter().map(|p| p.split_whitespace().collect()).collect();
    let common: HashSet<&str> = match sets.split_first() {
        Some((first, rest)) => first.iter().filter(|w| rest.iter().all(|s| s.contains(*w))).copied().collect(),
        None => HashSet::new(),
    };
    for p in phrases {
        let present = if isolated { text == p } else { format!(" {text} ").contains(&format!(" {p} ")) };
        if !present {
            continue;
        }
        let parts: Vec<&str> = p.split_whitespace().collect();
        let ok: Vec<bool> = words
            .iter()
            .filter(|w| parts.contains(&w["word"].as_str().unwrap_or("")))
            .map(|w| {
                let word = w["word"].as_str().unwrap_or("");
                let need = if common.contains(word) { min_conf } else { min_conf / phrases.len() as f64 };
                w["conf"].as_f64().unwrap_or(0.0) >= need
            })
            .collect();
        if !ok.is_empty() && ok.iter().all(|x| *x) {
            return true;
        }
    }
    false
}

pub fn is_end_word(word: &str, ends: &[String], threshold: f64) -> bool {
    ends.iter().any(|w| {
        let r = ratio(word, w);
        let same_start = word.chars().take(2).eq(w.chars().take(2));
        r >= threshold || (same_start && r >= threshold - 0.1)
    })
}

pub fn ends_with(text: &str, ends: &[String], threshold: f64) -> bool {
    text.split_whitespace().last().is_some_and(|w| is_end_word(w, ends, threshold))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn phrases() -> Vec<String> {
        vec!["aha ja".into(), "acha ja".into(), "a ha ja".into()]
    }

    #[test]
    fn wake_word_needs_confidence_and_isolation() {
        let r = json!({"text": "aha ja", "result": [{"word": "aha", "conf": 0.4}, {"word": "ja", "conf": 0.9}]});
        assert!(wake_heard(&r, &phrases(), 0.8, true));
        let weak = json!({"text": "aha ja", "result": [{"word": "aha", "conf": 0.4}, {"word": "ja", "conf": 0.5}]});
        assert!(!wake_heard(&weak, &phrases(), 0.8, true));
        let crowd = json!({"text": "[unk] aha ja", "result": [{"word": "aha", "conf": 1.0}, {"word": "ja", "conf": 1.0}]});
        assert!(!wake_heard(&crowd, &phrases(), 0.8, true));
        assert!(wake_heard(&crowd, &phrases(), 0.8, false));
    }

    #[test]
    fn end_word_is_fuzzy() {
        let ends = vec!["wykonaj".to_string()];
        assert!(ends_with("otwórz kalkulator wykonaj", &ends, 0.7));
        assert!(ends_with("otwórz kalkulator wykonanie", &ends, 0.7));
        assert!(!ends_with("otwórz kalkulator", &ends, 0.7));
    }
}
