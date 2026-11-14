//! `achaja claude stats`: the statistics in a few sentences for Achaja to say (Polish, like the
//! other session tools), and the printed table behind them.

use super::stats::{Stats, Totals};

fn tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1} mln", n as f64 / 1_000_000.0).replace('.', ",")
    } else if n >= 1000 {
        format!("{} tys.", (n as f64 / 1000.0).round())
    } else {
        n.to_string()
    }
}

/// "1 uruchomienie", "3 uruchomienia", "5 uruchomień" (Polish plural forms).
pub fn runs(n: u64) -> String {
    let word = if n == 1 {
        "uruchomienie"
    } else if (2..=4).contains(&(n % 10)) && !(12..=14).contains(&(n % 100)) {
        "uruchomienia"
    } else {
        "uruchomień"
    };
    format!("{n} {word}")
}

fn usd(v: f64) -> String {
    format!("{v:.2} USD").replace('.', ",")
}

fn sentence(label: &str, t: &Totals) -> String {
    if t.count == 0 {
        return format!("{label}: brak zapisanej pracy.");
    }
    let errors = if t.errors > 0 { format!(", w tym {} z błędem", t.errors) } else { String::new() };
    let time = if t.avg_ms > 0 { format!(", średni czas odpowiedzi {:.0} s", t.avg_ms as f64 / 1000.0) } else { String::new() };
    format!("{label}: {}{errors}, {} tokenów, szacunkowo {} w cenach API{time}.", runs(t.count), tokens(t.tokens.total()), usd(t.cost))
}

/// Today, the week and the most expensive item of the week.
pub fn summary(s: &Stats) -> String {
    let mut parts = vec![sentence("Dzisiaj", &s.today), sentence("Ostatnie 7 dni", &s.week)];
    if let Some(top) = s.top.first().filter(|t| t.cost > 0.0) {
        let what = match top.kind.as_str() {
            "job" => format!("zadanie w tle {}", top.name),
            "agent" => format!("agent projektu {}", top.name),
            "mail" => format!("sortowanie poczty ({})", top.name),
            "phone" => format!("powiadomienia na telefon ({})", top.name),
            _ => "rozmowa z Achają".to_string(),
        };
        parts.push(format!("Najdroższe w tym tygodniu: {what}, {}.", usd(top.cost)));
    }
    parts.push("To szacunek: plan Claude to abonament, a nie płatność za tokeny.".into());
    parts.join(" ")
}

/// The full table for the screen (`--details`).
pub fn details(s: &Stats) -> String {
    let mut out = vec![summary(s), String::new(), "Rodzaj / model (7 dni):".into()];
    for r in s.kinds.iter().chain(s.models.iter()) {
        out.push(format!("  {:<28} {:>4}x  błędy {:>2}  {:>10} tok.  {:>12}  śr. {:.1} s", r.key, r.count, r.errors, tokens(r.tokens), usd(r.cost), r.avg_ms as f64 / 1000.0));
    }
    out.push(String::new());
    out.push("Dni:".into());
    for d in &s.days {
        out.push(format!("  {}  {:>4}x  {:>10} tok.  {}", d.date, d.count, tokens(d.tokens), usd(d.cost)));
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speaks_counts_and_cost() {
        let mut s = Stats::default();
        assert!(summary(&s).starts_with("Dzisiaj: brak zapisanej pracy."));
        s.today = Totals { count: 3, errors: 1, cost: 1.234, avg_ms: 4200, ..Default::default() };
        s.today.tokens.output = 2_500_000;
        let text = summary(&s);
        assert!(text.contains("3 uruchomienia, w tym 1 z błędem, 2,5 mln tokenów, szacunkowo 1,23 USD"), "{text}");
        assert!(text.contains("średni czas odpowiedzi 4 s"));
        assert!(details(&s).contains("Dni:"));
        assert_eq!([runs(1), runs(3), runs(5), runs(12), runs(22)].join("|"), "1 uruchomienie|3 uruchomienia|5 uruchomień|12 uruchomień|22 uruchomienia");
    }
}
