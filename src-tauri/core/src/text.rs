//! Text helpers shared by the voice, mail and home code: comparison without case and Polish
//! letters, and Python's difflib similarity (the listener's thresholds were tuned with it).

use unicode_normalization::UnicodeNormalization;

/// Lower case without diacritics: "Głośnik" == "glosnik" ('ł' has no decomposition in NFKD).
pub fn fold(text: &str) -> String {
    text.to_lowercase()
        .replace('ł', "l")
        .nfkd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .collect()
}

/// Matching characters of the longest common blocks, as difflib.SequenceMatcher counts them
/// (no junk heuristics: the strings compared here are short words and names).
fn matching_chars(a: &[char], b: &[char]) -> usize {
    if a.is_empty() || b.is_empty() {
        return 0;
    }
    // Longest common substring; the earliest in a, then in b, wins ties (like difflib).
    let (mut best_i, mut best_j, mut best) = (0usize, 0usize, 0usize);
    let mut prev = vec![0usize; b.len() + 1];
    for i in 0..a.len() {
        let mut cur = vec![0usize; b.len() + 1];
        for j in 0..b.len() {
            if a[i] == b[j] {
                cur[j + 1] = prev[j] + 1;
                let k = cur[j + 1];
                if k > best {
                    best = k;
                    best_i = i + 1 - k;
                    best_j = j + 1 - k;
                }
            }
        }
        prev = cur;
    }
    if best == 0 {
        return 0;
    }
    best + matching_chars(&a[..best_i], &b[..best_j]) + matching_chars(&a[best_i + best..], &b[best_j + best..])
}

/// difflib.SequenceMatcher(None, a, b).ratio()
pub fn ratio(a: &str, b: &str) -> f64 {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let total = a.len() + b.len();
    if total == 0 {
        return 1.0;
    }
    2.0 * matching_chars(&a, &b) as f64 / total as f64
}

/// difflib.get_close_matches(word, candidates, n=1, cutoff): the best candidate or None.
pub fn close_match<'a>(word: &str, candidates: impl IntoIterator<Item = &'a str>, cutoff: f64) -> Option<&'a str> {
    let mut best: Option<(f64, &str)> = None;
    for cand in candidates {
        let score = ratio(cand, word);
        if score >= cutoff && best.map_or(true, |(s, _)| score > s) {
            best = Some((score, cand));
        }
    }
    best.map(|(_, c)| c)
}

/// Collapses whitespace runs into single spaces.
pub fn squash(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// At most `limit` characters (not bytes), with "..." when cut.
pub fn short(text: &str, limit: usize) -> String {
    let flat = squash(text);
    if flat.chars().count() <= limit {
        return flat;
    }
    format!("{}...", flat.chars().take(limit).collect::<String>())
}

pub fn truncate_chars(text: &str, limit: usize) -> String {
    text.chars().take(limit).collect()
}

/// First letter upper case.
pub fn cap(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
