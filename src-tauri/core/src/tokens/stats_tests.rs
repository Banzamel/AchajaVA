use std::collections::BTreeMap;

use chrono::{Local, TimeZone};

use super::*;

fn entry(at: f64, kind: &str, name: &str, model: &str, out: u64, ms: u64, ok: bool) -> Entry {
    let mut models = BTreeMap::new();
    models.insert(model.to_string(), Tokens { output: out, ..Default::default() });
    Entry { at, kind: kind.into(), name: name.into(), session: if kind == "session" { "s1".into() } else { String::new() }, models, ms, turns: 1, ok, effort: if kind == "session" { "low".into() } else { String::new() }, retries: 0 }
}

#[test]
fn sums_today_week_trend_and_top() {
    let now = Local.with_ymd_and_hms(2026, 11, 14, 12, 0, 0).single().expect("valid date");
    let t = now.timestamp() as f64;
    let day = 86400.0;
    let list = vec![
        entry(t - 60.0, "session", "", "claude-sonnet-5-5", 1_000_000, 2000, true),
        entry(t - 120.0, "session", "", "claude-sonnet-5-5", 1_000_000, 4000, false),
        entry(t - 2.0 * day, "job", "poczta", "claude-opus-5-5", 3_000_000, 10_000, true),
        entry(t - 10.0 * day, "agent", "shop", "claude-opus-5-5", 1_000_000, 0, true),
    ];
    let s = build(&list, &Prices::defaults(), now);
    assert_eq!((s.today.count, s.today.errors), (2, 1));
    assert!((s.today.cost - 20.0).abs() < 1e-9);
    assert_eq!((s.today.avg_ms, s.today.p90_ms), (3000, 4000));
    assert_eq!(s.week.count, 3);
    assert_eq!(s.days.len(), TREND_DAYS as usize);
    assert_eq!(s.days.last().map(|d| d.count), Some(2));
    assert_eq!(s.days.iter().map(|d| d.count).sum::<u64>(), 4);
    assert_eq!(s.kinds[0].key, "job");
    assert_eq!(s.models.iter().map(|r| r.key.as_str()).collect::<Vec<_>>(), ["claude-opus-5-5", "claude-sonnet-5-5 · low"]);
    assert_eq!(s.top.len(), 2);
    assert_eq!((s.top[0].kind.as_str(), s.top[0].count), ("job", 1));
    assert_eq!(s.top[1].count, 2);
}
