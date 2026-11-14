//! Statistics of the ledger for the Claude tab and `achaja claude stats`: today and the last 7
//! days (commands, errors, tokens, estimated cost, response time), the split by kind and by
//! model (with the session's effort, to compare e.g. sonnet+low with opus+medium), a daily
//! trend of 14 days and the most expensive sessions, jobs and agents of the week.

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Local, NaiveDate, TimeZone};
use serde::Serialize;

use super::ledger::{self, Entry};
use super::price::Prices;
use super::Tokens;
use crate::paths::Paths;

pub const TREND_DAYS: i64 = 14;

#[derive(Debug, Default, Serialize)]
pub struct Totals {
    pub count: u64,
    pub errors: u64,
    pub tokens: Tokens,
    pub cost: f64,
    pub avg_ms: u64,
    pub p90_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct Row {
    pub key: String,
    pub count: u64,
    pub errors: u64,
    pub tokens: u64,
    pub cost: f64,
    pub avg_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct Day {
    pub date: String,
    pub count: u64,
    pub tokens: u64,
    pub cost: f64,
}

#[derive(Debug, Serialize)]
pub struct Top {
    pub kind: String,
    pub name: String,
    pub session: String,
    pub last: f64,
    pub count: u64,
    pub tokens: u64,
    pub cost: f64,
}

#[derive(Debug, Default, Serialize)]
pub struct Stats {
    pub today: Totals,
    pub week: Totals,
    pub kinds: Vec<Row>,
    pub models: Vec<Row>,
    pub days: Vec<Day>,
    pub top: Vec<Top>,
    pub retries: u64,
}

pub fn day_of(at: f64) -> NaiveDate {
    Local.timestamp_opt(at as i64, 0).single().map(|d| d.date_naive()).unwrap_or_default()
}

fn cost(e: &Entry, p: &Prices) -> f64 {
    e.models.iter().map(|(m, t)| p.cost(m, t)).sum()
}

fn totals(list: &[&Entry], p: &Prices) -> Totals {
    let mut t = Totals { count: list.len() as u64, ..Default::default() };
    let mut times: Vec<u64> = Vec::new();
    for e in list {
        t.errors += u64::from(!e.ok);
        t.tokens.add(&e.tokens());
        t.cost += cost(e, p);
        if e.ms > 0 {
            times.push(e.ms);
        }
    }
    times.sort_unstable();
    if !times.is_empty() {
        t.avg_ms = times.iter().sum::<u64>() / times.len() as u64;
        t.p90_ms = times[((times.len() as f64 * 0.9).ceil() as usize).clamp(1, times.len()) - 1];
    }
    t
}

fn rows(list: &[&Entry], p: &Prices, key: impl Fn(&Entry) -> String) -> Vec<Row> {
    let mut groups: BTreeMap<String, Vec<&Entry>> = BTreeMap::new();
    for &e in list {
        groups.entry(key(e)).or_default().push(e);
    }
    let mut out: Vec<Row> = groups
        .into_iter()
        .map(|(key, g)| {
            let t = totals(&g, p);
            Row { key, count: t.count, errors: t.errors, tokens: t.tokens.total(), cost: t.cost, avg_ms: t.avg_ms }
        })
        .collect();
    out.sort_by(|a, b| b.cost.total_cmp(&a.cost).then(b.count.cmp(&a.count)));
    out
}

fn model_key(e: &Entry) -> String {
    let m = e.main_model();
    if e.effort.is_empty() { m.to_string() } else { format!("{m} · {}", e.effort) }
}

fn top(list: &[&Entry], p: &Prices) -> Vec<Top> {
    let mut groups: BTreeMap<(String, String), Top> = BTreeMap::new();
    for e in list {
        let id = if e.session.is_empty() { e.name.clone() } else { e.session.clone() };
        let t = groups.entry((e.kind.clone(), id)).or_insert_with(|| Top { kind: e.kind.clone(), name: e.name.clone(), session: e.session.clone(), last: 0.0, count: 0, tokens: 0, cost: 0.0 });
        t.last = t.last.max(e.at);
        t.count += 1;
        t.tokens += e.tokens().total();
        t.cost += cost(e, p);
    }
    let mut out: Vec<Top> = groups.into_values().collect();
    out.sort_by(|a, b| b.cost.total_cmp(&a.cost).then(b.tokens.cmp(&a.tokens)));
    out.truncate(5);
    out
}

pub fn build(entries: &[Entry], p: &Prices, now: DateTime<Local>) -> Stats {
    let today = now.date_naive();
    let week_start = today - Duration::days(6);
    let in_week: Vec<&Entry> = entries.iter().filter(|e| day_of(e.at) >= week_start).collect();
    let in_today: Vec<&Entry> = in_week.iter().copied().filter(|e| day_of(e.at) == today).collect();
    let days = (0..TREND_DAYS)
        .rev()
        .map(|back| {
            let d = today - Duration::days(back);
            let of_day: Vec<&Entry> = entries.iter().filter(|e| day_of(e.at) == d).collect();
            let t = totals(&of_day, p);
            Day { date: d.format("%Y-%m-%d").to_string(), count: t.count, tokens: t.tokens.total(), cost: t.cost }
        })
        .collect();
    Stats {
        today: totals(&in_today, p),
        week: totals(&in_week, p),
        kinds: rows(&in_week, p, |e| e.kind.clone()),
        models: rows(&in_week, p, model_key),
        days,
        top: top(&in_week, p),
        retries: in_week.iter().map(|e| e.retries).sum(),
    }
}

/// The statistics of the ledger with the prices from config.json.
pub fn load(paths: &Paths) -> Stats {
    let since = (Local::now() - Duration::days(TREND_DAYS + 1)).timestamp() as f64;
    let prices = Prices::from_config(&crate::config::load_or_default(paths));
    build(&ledger::read(paths, since), &prices, Local::now())
}

/// The estimated cost of today's work (the daily cost limit).
pub fn cost_today(paths: &Paths) -> f64 {
    let start = Local::now().date_naive().and_hms_opt(0, 0, 0).and_then(|d| Local.from_local_datetime(&d).single()).map(|d| d.timestamp() as f64).unwrap_or(0.0);
    let prices = Prices::from_config(&crate::config::load_or_default(paths));
    ledger::read(paths, start).iter().map(|e| cost(e, &prices)).sum()
}

#[cfg(test)]
#[path = "stats_tests.rs"]
mod tests;
