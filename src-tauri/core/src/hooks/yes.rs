//! Is the user's prompt a "yes" to what Achaja asked? Only the user's own dictated prompt can
//! approve a gated action; a scheduled command ([ZAPLANOWANE]) never does.

use std::sync::OnceLock;

use regex::Regex;

use crate::text::fold;

pub const SCHEDULED: &str = "[ZAPLANOWANE]";

const YES: &[&str] = &[
    "tak", "yes", "potwierdzam", "zgoda", "zgadzam", "dawaj", "jasne", "oczywiscie", "ok", "okej", "okay", "dobrze", "rob",
    "zrob", "wyslij", "usun", "smialo", "sure", "confirm", "confirmed", "yep", "yeah", "go", "proceed", "send", "delete",
    "do", "wypchnij", "push", "otworz", "zamknij", "instaluj", "zainstaluj",
];
const NO: &[&str] = &["nie", "no", "not", "dont", "stop", "anuluj", "cancel", "czekaj", "wait", "nigdy", "never"];

/// `ends`: the end words ("wykonaj") that may close a dictated prompt.
pub fn is_yes(prompt: &str, ends: &[String]) -> bool {
    static WORDS: OnceLock<Option<Regex>> = OnceLock::new();
    if prompt.trim().is_empty() || prompt.trim_start().starts_with(SCHEDULED) {
        return false;
    }
    let mut text = fold(prompt);
    let mut folded: Vec<String> = ends.iter().map(|e| fold(e)).collect();
    folded.sort_by_key(|e| std::cmp::Reverse(e.len()));
    for end in folded.iter().filter(|e| !e.is_empty()) {
        if let Ok(re) = Regex::new(&format!(r"\b{}\W*$", regex::escape(end))) {
            text = re.replace(text.trim(), "").into_owned();
        }
    }
    let Some(re) = WORDS.get_or_init(|| Regex::new("[a-z]+").ok()).as_ref() else { return false };
    let cleaned = text.replace('\'', "");
    let mut words: Vec<&str> = re.find_iter(&cleaned).map(|m| m.as_str()).collect();
    // Polish "no tak", "no dobrze"
    if words.len() > 1 && ["no", "noo"].contains(&words[0]) && ["tak", "dobrze", "jasne", "dawaj", "okej", "ok"].contains(&words[1]) {
        words.remove(0);
    }
    if words.is_empty() || words.len() > 12 || words.iter().any(|w| NO.contains(w)) {
        return false;
    }
    YES.contains(&words[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ends() -> Vec<String> {
        vec!["wykonaj".into(), "execute".into()]
    }

    #[test]
    fn recognizes_yes_and_no() {
        assert!(is_yes("Tak, wyślij wykonaj", &ends()));
        assert!(is_yes("no tak", &ends()));
        assert!(is_yes("Yes please execute.", &ends()));
        assert!(!is_yes("nie, nie wysyłaj", &ends()));
        assert!(!is_yes("tak ale nie teraz", &ends()));
        assert!(!is_yes("[ZAPLANOWANE] tak", &ends()));
        assert!(!is_yes("co nowego w poczcie", &ends()));
    }
}
