//! The spoken mail report: important mail with sender and gist, the rest as numbers ("I removed
//! 30 ads and 2 spam messages"), in Polish with the right plural forms, or in English.

use super::log::Entry;
use super::neutral::{neutral, no_wake, speak_name};
use crate::text::cap;

pub fn label(category: &str, lang: &str) -> String {
    let pl = [("important", "ważne"), ("normal", "zwykłe"), ("ad", "reklama"), ("spam", "spam"), ("threat", "zagrożenie")];
    if lang == "pl" {
        pl.iter().find(|(k, _)| *k == category).map(|(_, v)| v.to_string()).unwrap_or_else(|| category.to_string())
    } else {
        category.to_string()
    }
}

fn forms(category: &str, lang: &str) -> (&'static str, &'static str, &'static str) {
    match (lang, category) {
        ("pl", "important") => ("jeden ważny mail", "ważne maile", "ważnych maili"),
        ("pl", "normal") => ("jeden zwykły mail", "zwykłe maile", "zwykłych maili"),
        ("pl", "ad") => ("jedną reklamę", "reklamy", "reklam"),
        ("pl", "spam") => ("jeden spam", "spamy", "spamów"),
        ("pl", _) => ("jedną podejrzaną wiadomość", "podejrzane wiadomości", "podejrzanych wiadomości"),
        (_, "important") => ("one important e-mail", "important e-mails", "important e-mails"),
        (_, "normal") => ("one ordinary e-mail", "ordinary e-mails", "ordinary e-mails"),
        (_, "ad") => ("one ad", "ads", "ads"),
        (_, "spam") => ("one spam message", "spam messages", "spam messages"),
        _ => ("one suspicious e-mail", "suspicious e-mails", "suspicious e-mails"),
    }
}

pub fn count_phrase(n: usize, category: &str, lang: &str) -> String {
    let (one, few, many) = forms(category, lang);
    if n == 1 {
        return one.to_string();
    }
    if lang == "pl" && (2..=4).contains(&(n % 10)) && !(12..=14).contains(&(n % 100)) {
        return format!("{n} {few}");
    }
    format!("{n} {many}")
}

fn join_words(items: &[String], lang: &str) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        n => format!("{}{}{}", items[..n - 1].join(", "), if lang == "pl" { " i " } else { " and " }, items[n - 1]),
    }
}

pub struct ReportOptions<'a> {
    pub lang: &'a str,
    pub intro: &'a str,
    pub names: usize,
    pub multi_account: bool,
    pub wake_phrases: &'a [String],
}

pub fn report(entries: &[Entry], o: &ReportOptions) -> String {
    let pl = o.lang == "pl";
    let by = |c: &str| entries.iter().filter(|e| e.category == c).collect::<Vec<_>>();
    let mut parts: Vec<String> = if o.intro.is_empty() { Vec::new() } else { vec![o.intro.to_string()] };
    let src = |e: &Entry| {
        let place = if o.multi_account { if pl { format!(" na koncie {}", e.account) } else { format!(" in {}", e.account) } } else { String::new() };
        let gist = if e.summary.is_empty() { &e.subject } else { &e.summary };
        let about = no_wake(&neutral(gist, true), o.wake_phrases).trim_end_matches('.').to_string();
        let who = speak_name(&e.from_name, &e.from_addr);
        let who = Some(no_wake(&who, o.wake_phrases)).filter(|w| !w.is_empty()).unwrap_or(who);
        format!("{} {who}{place}: {about}", if pl { "od" } else { "from" })
    };
    let mut imp = by("important");
    imp.sort_by_key(|e| !e.unread); // unread first
    if imp.len() == 1 {
        parts.push(format!("{}{}.", if pl { "Ważny mail " } else { "Important e-mail " }, src(imp[0])));
    } else if !imp.is_empty() {
        parts.push(format!("{}.", cap(&count_phrase(imp.len(), "important", o.lang))));
        parts.extend(imp.iter().take(o.names).map(|e| format!("{}.", cap(&src(e)))));
        if imp.len() > o.names {
            let more = imp.len() - o.names;
            parts.push(if pl { format!("I jeszcze {more}.") } else { format!("And {more} more.") });
        }
    }
    let removed: Vec<String> = ["ad", "spam"].iter().filter(|c| !by(c).is_empty()).map(|c| count_phrase(by(c).len(), c, o.lang)).collect();
    if !removed.is_empty() {
        parts.push(format!("{}{}.", if pl { "Usunęłam ze skrzynki " } else { "I removed from the inbox " }, join_words(&removed, o.lang)));
    }
    let threats = by("threat");
    if !threats.is_empty() {
        let detail = if threats.len() == 1 { format!(", {}", src(threats[0])) } else { String::new() };
        parts.push(format!("{}{}{detail}.", if pl { "Odizolowałam " } else { "I quarantined " }, count_phrase(threats.len(), "threat", o.lang)));
    }
    let normal = by("normal");
    if !normal.is_empty() {
        parts.push(format!("{}{}", cap(&count_phrase(normal.len(), "normal", o.lang)), if pl { " zostawiłam w skrzynce." } else { " left in the inbox." }));
    }
    if parts.len() == usize::from(!o.intro.is_empty()) {
        parts.push(if pl { "Nic nowego w poczcie.".into() } else { "Nothing new in the mail.".into() });
    }
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(cat: &str, name: &str, summary: &str) -> Entry {
        Entry { category: cat.into(), from_name: name.into(), summary: summary.into(), unread: true, ..Default::default() }
    }

    #[test]
    fn polish_report() {
        let entries = vec![e("important", "Kasia", "Pyta o spotkanie."), e("ad", "", ""), e("ad", "", ""), e("spam", "", "")];
        let o = ReportOptions { lang: "pl", intro: "Poczta.", names: 3, multi_account: false, wake_phrases: &[] };
        assert_eq!(report(&entries, &o), "Poczta. Ważny mail od Kasia: Pyta o spotkanie. Usunęłam ze skrzynki 2 reklamy i jeden spam.");
        assert_eq!(count_phrase(12, "ad", "pl"), "12 reklam");
        assert_eq!(count_phrase(22, "spam", "pl"), "22 spamy");
        let empty = ReportOptions { lang: "en", intro: "", names: 3, multi_account: false, wake_phrases: &[] };
        assert_eq!(report(&[], &empty), "Nothing new in the mail.");
    }
}
