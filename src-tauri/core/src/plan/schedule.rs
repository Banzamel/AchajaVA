//! When a planned item runs: once (--in 30m, --at 15:30 or "2026-11-20 09:00") or repeatedly
//! ("daily 08:00", "weekdays 08:00", "weekly 4 16:00" with Monday = 0). Local time.

use chrono::{Datelike, Duration, Local, NaiveDateTime, TimeZone};

use crate::error::{AppError, Result};

pub const DAY_NAMES: [&str; 7] = ["pon", "wt", "sr", "czw", "pt", "sob", "nd"];

fn err(text: &str) -> AppError {
    AppError::msg(text)
}

pub fn hhmm(text: &str) -> Result<(u32, u32)> {
    let t = text.trim();
    let (h, m) = t.split_once([':', '.']).ok_or_else(|| err(&format!("Godzina w formacie GG:MM (jest: '{text}').")))?;
    let (h, m): (u32, u32) = (h.parse().map_err(|_| err("Godzina w formacie GG:MM."))?, m.parse().map_err(|_| err("Godzina w formacie GG:MM."))?);
    if h > 23 || m > 59 || t.split_once([':', '.']).map_or(true, |(_, mm)| mm.len() != 2) {
        return Err(err(&format!("Godzina w formacie GG:MM (jest: '{text}').")));
    }
    Ok((h, m))
}

fn at(day: NaiveDateTime, h: u32, m: u32) -> NaiveDateTime {
    day.date().and_hms_opt(h, m, 0).unwrap_or(day)
}

/// The next time of a repeated item after `after`.
pub fn next_time(repeat: &str, after: NaiveDateTime) -> Result<NaiveDateTime> {
    let (kind, spec) = repeat.split_once(' ').unwrap_or((repeat, ""));
    if kind == "weekly" {
        let (day, clock) = spec.split_once(' ').unwrap_or((spec, ""));
        let day: i64 = day.parse().map_err(|_| err("Zly dzien tygodnia."))?;
        let (h, m) = hhmm(clock)?;
        let shift = (day - i64::from(after.weekday().num_days_from_monday())).rem_euclid(7);
        let cand = at(after, h, m) + Duration::days(shift);
        return Ok(if cand > after { cand } else { cand + Duration::days(7) });
    }
    let (h, m) = hhmm(spec)?;
    let mut cand = at(after, h, m);
    while cand <= after || (kind == "weekdays" && cand.weekday().num_days_from_monday() > 4) {
        cand += Duration::days(1);
    }
    Ok(cand)
}

#[derive(Debug, Default, Clone)]
pub struct When {
    pub in_: Option<String>,
    pub at: Option<String>,
    pub daily: Option<String>,
    pub weekdays: Option<String>,
    pub weekly: Option<String>,
}

fn day_index(word: &str) -> Option<u32> {
    let key: String = word.to_lowercase().replace('ś', "s").replace('ą', "a").replace('ę', "e").chars().take(3).collect();
    let key = key.trim_end_matches('.');
    let table = [
        ("pon", 0), ("pn", 0), ("mon", 0), ("wt", 1), ("wto", 1), ("tue", 1), ("sr", 2), ("sro", 2), ("wed", 2), ("czw", 3),
        ("thu", 3), ("pt", 4), ("pia", 4), ("fri", 4), ("sob", 5), ("so", 5), ("sat", 5), ("nd", 6), ("nie", 6), ("ndz", 6), ("sun", 6),
    ];
    let find = |k: &str| table.iter().find(|(n, _)| *n == k).map(|(_, i)| *i);
    find(key).or_else(|| find(&key.chars().take(2).collect::<String>()))
}

/// (first time, repeat rule or None).
pub fn parse_when(w: &When, now: NaiveDateTime) -> Result<(NaiveDateTime, Option<String>)> {
    if let Some(spec) = &w.in_ {
        let s = spec.to_lowercase();
        let re = regex::Regex::new(r"(\d+)\s*([dhm])")?;
        let secs: i64 = re.captures_iter(&s).map(|c| c[1].parse::<i64>().unwrap_or(0) * match &c[2] { "d" => 86400, "h" => 3600, _ => 60 }).sum();
        let rest = regex::Regex::new(r"[\d\sdhm]")?.replace_all(&s, "");
        if secs == 0 || !rest.is_empty() {
            return Err(err("--in w formacie 30m, 2h, 1d albo 1h30m."));
        }
        return Ok((now + Duration::seconds(secs), None));
    }
    if let Some(spec) = &w.at {
        let t = spec.trim();
        if regex::Regex::new(r"^\d{1,2}[:.]\d{2}$")?.is_match(t) {
            let (h, m) = hhmm(t)?;
            let cand = at(now, h, m);
            return Ok((if cand > now { cand } else { cand + Duration::days(1) }, None));
        }
        let parsed = NaiveDateTime::parse_from_str(&t.replace('T', " "), "%Y-%m-%d %H:%M").map_err(|_| err("--at w formacie GG:MM albo 'RRRR-MM-DD GG:MM'."))?;
        return Ok((parsed, None));
    }
    let repeat = if let Some(c) = &w.daily {
        let (h, m) = hhmm(c)?;
        format!("daily {h:02}:{m:02}")
    } else if let Some(c) = &w.weekdays {
        let (h, m) = hhmm(c)?;
        format!("weekdays {h:02}:{m:02}")
    } else if let Some(spec) = &w.weekly {
        let (day, clock) = spec.trim().split_once(' ').unwrap_or((spec.trim(), ""));
        let d = day_index(day).ok_or_else(|| err("--weekly 'pon 09:00' (pon wt sr czw pt sob nd)."))?;
        let (h, m) = hhmm(clock)?;
        format!("weekly {d} {h:02}:{m:02}")
    } else {
        return Err(err("Podaj kiedy: --in, --at, --daily, --weekdays albo --weekly."));
    };
    Ok((next_time(&repeat, now)?, Some(repeat)))
}

pub fn describe_repeat(repeat: Option<&str>) -> String {
    let Some(repeat) = repeat else { return "jednorazowo".into() };
    let (kind, spec) = repeat.split_once(' ').unwrap_or((repeat, ""));
    match kind {
        "weekly" => {
            let (day, clock) = spec.split_once(' ').unwrap_or((spec, ""));
            let name = day.parse::<usize>().ok().and_then(|d| DAY_NAMES.get(d)).unwrap_or(&"?");
            format!("co tydzien ({name} {clock})")
        }
        "daily" => format!("codziennie o {spec}"),
        _ => format!("w dni robocze o {spec}"),
    }
}

pub fn to_ts(t: NaiveDateTime) -> f64 {
    Local.from_local_datetime(&t).earliest().map(|d| d.timestamp() as f64).unwrap_or(0.0)
}

pub fn from_ts(ts: f64) -> NaiveDateTime {
    Local.timestamp_opt(ts as i64, 0).earliest().map(|d| d.naive_local()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()
    }

    #[test]
    fn repeats() {
        // 2026-11-06 is a Friday
        assert_eq!(next_time("daily 08:00", t("2026-11-06 09:00")).unwrap(), t("2026-11-07 08:00"));
        assert_eq!(next_time("weekdays 08:00", t("2026-11-06 09:00")).unwrap(), t("2026-11-09 08:00"));
        assert_eq!(next_time("weekly 4 16:00", t("2026-11-06 15:00")).unwrap(), t("2026-11-06 16:00"));
        assert_eq!(next_time("weekly 4 16:00", t("2026-11-06 17:00")).unwrap(), t("2026-11-13 16:00"));
    }

    #[test]
    fn parses_when() {
        let now = t("2026-11-06 15:00");
        let w = |f: fn(&mut When)| {
            let mut x = When::default();
            f(&mut x);
            parse_when(&x, now).unwrap()
        };
        assert_eq!(w(|x| x.in_ = Some("1h30m".into())).0, t("2026-11-06 16:30"));
        assert_eq!(w(|x| x.at = Some("14:00".into())).0, t("2026-11-07 14:00"));
        assert_eq!(w(|x| x.weekly = Some("pt 16:00".into())).1.as_deref(), Some("weekly 4 16:00"));
        assert_eq!(w(|x| x.weekly = Some("środa 9:30".into())).1.as_deref(), Some("weekly 2 09:30"));
        assert!(parse_when(&When { in_: Some("soon".into()), ..Default::default() }, now).is_err());
    }
}
