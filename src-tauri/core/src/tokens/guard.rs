//! Limits: a spoken warning once the plan's usage window passes limits.warn_at percent (once per
//! window and reset, so it is not repeated every check), and recurring background jobs that do
//! not start while a main window (5 hours, 7 days) is past limits.pause_jobs_at or today's
//! estimated cost is past limits.daily_cost_usd. Voice commands are never blocked.

use serde_json::{Map, Value};

use crate::claude::account::{cached_usage, usage, UsageWindow};
use crate::claude::labels::{reset_text, window_label};
use crate::config::{self, Config};
use crate::paths::Paths;
use crate::state;

const WARNED: &str = "limits_warned.json"; // {window key: resets_at already warned about}
const MAIN: &[&str] = &["five_hour", "seven_day"];

/// The sentence to speak for the fullest window past the threshold not yet warned about.
pub fn plan_warning(paths: &Paths, cfg: &Config, windows: &[UsageWindow]) -> Option<String> {
    let at = cfg.f64_or("limits.warn_at", 95.0);
    if at <= 0.0 {
        return None;
    }
    let mut warned = match state::read(paths, WARNED) {
        Value::Object(m) => m,
        _ => Map::new(),
    };
    let reset = |w: &UsageWindow| Value::String(w.resets_at.clone().unwrap_or_default());
    let w = windows.iter().filter(|w| w.percent >= at && warned.get(&w.key) != Some(&reset(w))).max_by(|a, b| a.percent.total_cmp(&b.percent))?;
    warned.insert(w.key.clone(), reset(w));
    state::write(paths, WARNED, &Value::Object(warned));
    let lang = cfg.lang2();
    let when = reset_text(w.resets_at.as_deref(), lang, chrono::Local::now());
    let when = if when.is_empty() { String::new() } else { format!(", {when}") };
    let (pct, name) = (w.percent.round(), window_label(&w.key, lang));
    Some(if lang == "en" {
        format!("Heads up: {pct} percent of the Claude limit for {name} is used{when}.")
    } else {
        format!("Uwaga: wykorzystane jest już {pct} procent limitu Claude na {name}{when}.")
    })
}

/// Fetches the plan's usage and speaks a warning when one is due (the listener runs it every
/// few minutes in its own thread: the request may take a few seconds).
pub fn check_plan(paths: &Paths) {
    let cfg = config::load_or_default(paths);
    let Ok(windows) = crate::runtime::block_on(usage(paths, false)) else { return };
    if let Some(text) = plan_warning(paths, &cfg, &windows) {
        crate::voice::speak::say(paths, &text);
    }
}

/// Why a recurring job must not start now, or None.
pub fn jobs_paused(cfg: &Config, windows: &[UsageWindow], cost_today: f64) -> Option<String> {
    let at = cfg.f64_or("limits.pause_jobs_at", 95.0);
    if at > 0.0 {
        if let Some(w) = windows.iter().find(|w| MAIN.contains(&w.key.as_str()) && w.percent >= at) {
            return Some(format!("limit {} wykorzystany w {:.0}%", window_label(&w.key, "pl"), w.percent));
        }
    }
    let max = cfg.f64_or("limits.daily_cost_usd", 0.0);
    if max > 0.0 && cost_today >= max {
        return Some(format!("dzisiejszy szacowany koszt {cost_today:.2} USD przekroczył limit {max:.2} USD"));
    }
    None
}

/// jobs_paused() with the usage last read from disk and today's ledger.
pub fn jobs_paused_now(paths: &Paths) -> Option<String> {
    let cfg = config::load_or_default(paths);
    let cost = if cfg.f64_or("limits.daily_cost_usd", 0.0) > 0.0 { super::stats::cost_today(paths) } else { 0.0 };
    jobs_paused(&cfg, &cached_usage(paths), cost)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn w(key: &str, percent: f64, reset: &str) -> UsageWindow {
        UsageWindow { key: key.into(), percent, resets_at: Some(reset.into()) }
    }

    #[test]
    fn warns_once_per_window_and_reset() {
        let dir = std::env::temp_dir().join(format!("achaja-guard-{}", std::process::id()));
        let paths = Paths::at(&dir, &dir);
        let cfg = Config::new(json!({"language": "pl"}));
        let list = vec![w("five_hour", 96.4, "r1"), w("seven_day", 40.0, "r2")];
        let text = plan_warning(&paths, &cfg, &list).unwrap_or_default();
        assert!(text.contains("96 procent") && text.contains("5 godzin"), "{text}");
        assert_eq!(plan_warning(&paths, &cfg, &list), None);
        assert!(plan_warning(&paths, &cfg, &[w("five_hour", 97.0, "r3")]).is_some());
        assert_eq!(plan_warning(&paths, &Config::new(json!({"limits": {"warn_at": 0}})), &[w("seven_day", 99.0, "x")]), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pauses_jobs_on_main_windows_and_cost() {
        let cfg = Config::new(json!({"limits": {"daily_cost_usd": 10}}));
        assert_eq!(jobs_paused(&cfg, &[w("seven_day_opus", 99.0, "")], 1.0), None);
        assert!(jobs_paused(&cfg, &[w("seven_day", 95.0, "")], 0.0).is_some_and(|r| r.contains("7 dni")));
        assert!(jobs_paused(&cfg, &[], 12.5).is_some_and(|r| r.contains("12.50")));
    }
}
