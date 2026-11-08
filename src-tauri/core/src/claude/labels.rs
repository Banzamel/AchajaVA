//! Spoken and printed names of the usage windows and when they reset, and finding a project by a
//! dictated name ("achaja claude sessions <project>").

use chrono::{Datelike, Local, TimeZone};

use super::sessions::{projects, Project};
use crate::error::{AppError, Result};

pub fn window_label(key: &str, lang: &str) -> String {
    let pl = lang == "pl";
    for (prefix, name) in [("five_hour", if pl { "5 godzin" } else { "5 hours" }), ("seven_day", if pl { "7 dni" } else { "7 days" })] {
        if key == prefix {
            return name.to_string();
        }
        if let Some(rest) = key.strip_prefix(&format!("{prefix}_")) {
            return format!("{name} · {}", crate::text::cap(&rest.replace('_', " ")));
        }
    }
    key.replace('_', " ")
}

/// "odnowi się za 2 h 15 min" / "odnowi się w pt 14:00" (empty when unknown or past).
pub fn reset_text(iso: Option<&str>, lang: &str, now: chrono::DateTime<Local>) -> String {
    let Some(when) = iso.and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok()) else { return String::new() };
    let when = Local.from_utc_datetime(&when.naive_utc());
    let left = (when - now).num_seconds();
    if left <= 0 {
        return String::new();
    }
    let pl = lang == "pl";
    if left < 24 * 3600 {
        let (h, m) = (left / 3600, left % 3600 / 60);
        let span = if h > 0 { format!("{h} h {m} min") } else { format!("{m} min") };
        return if pl { format!("odnowi się za {span}") } else { format!("resets in {span}") };
    }
    let days = if pl { ["pon", "wt", "śr", "czw", "pt", "sob", "nd"] } else { ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"] };
    let day = days[when.weekday().num_days_from_monday() as usize];
    let clock = when.format("%H:%M");
    if pl {
        format!("odnowi się w {day} {clock}")
    } else {
        format!("resets {day} {clock}")
    }
}

pub fn find_project(query: &str) -> Result<Project> {
    let items = projects();
    if let Some(p) = items.iter().find(|p| p.id == query || p.path == query) {
        return Ok(p.clone());
    }
    let q = query.to_lowercase();
    let hits: Vec<&Project> = items.iter().filter(|p| p.path.to_lowercase().contains(&q) || p.id.to_lowercase().contains(&q)).collect();
    match hits.len() {
        1 => Ok(hits[0].clone()),
        0 => Err(AppError::msg(format!("Nie ma projektu '{query}' w ~/.claude/projects."))),
        _ => Err(AppError::msg(format!(
            "Kilka projektow pasuje do '{query}': {}",
            hits.iter().take(6).map(|p| p.path.as_str()).collect::<Vec<_>>().join(", ")
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_windows() {
        assert_eq!(window_label("five_hour", "pl"), "5 godzin");
        assert_eq!(window_label("seven_day_opus", "en"), "7 days · Opus");
    }

    #[test]
    fn reset_soon_and_later() {
        let now = Local::now();
        let soon = (now + chrono::Duration::minutes(135)).to_rfc3339();
        assert!(reset_text(Some(&soon), "pl", now).starts_with("odnowi się za 2 h 1"));
        assert_eq!(reset_text(Some("2000-01-01T00:00:00Z"), "pl", now), "");
    }
}
