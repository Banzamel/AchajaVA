use super::*;

fn line(id: &str, ts: &str, model: &str, out: u64) -> String {
    json!({"type": "assistant", "timestamp": ts, "message": {"id": id, "model": model, "usage": {"input_tokens": 1, "output_tokens": out}}}).to_string()
}

fn closing(ts: &str, ms: u64) -> String {
    json!({"type": "system", "subtype": "turn_duration", "durationMs": ms, "timestamp": ts}).to_string()
}

fn ts(text: &str) -> f64 {
    chrono::DateTime::parse_from_rfc3339(text).map(|t| t.timestamp() as f64).unwrap_or(0.0)
}

#[test]
fn counts_each_message_once_since_the_command() {
    let text = [
        line("old", "2026-11-14T10:00:00Z", "claude-sonnet-5-5", 100),
        line("a", "2026-11-14T10:05:00Z", "claude-sonnet-5-5", 10),
        line("a", "2026-11-14T10:05:01Z", "claude-sonnet-5-5", 10),
        line("b", "2026-11-14T10:05:02Z", "claude-haiku-5-5", 3),
        line("c", "2026-11-14T10:05:03Z", "<synthetic>", 0),
        "{\"type\":\"user\"}".to_string(),
        "not json".to_string(),
    ]
    .join("\n");
    let m = parse(&text, ts("2026-11-14T10:04:00Z"));
    assert_eq!(m.len(), 2);
    assert_eq!(m["claude-sonnet-5-5"], Tokens { input: 1, output: 10, ..Default::default() });
    assert_eq!(m["claude-haiku-5-5"].output, 3);
}

#[test]
fn finds_the_end_of_the_turn() {
    let old = closing("2026-11-14T10:00:00Z", 900);
    let text = format!("{old}\n{}\n{}\nrest\n", line("a", "2026-11-14T10:05:00Z", "x", 1), closing("2026-11-14T10:05:02Z", 3670));
    let (end, ms) = turn_end(&text, ts("2026-11-14T10:04:00Z")).unwrap_or_default();
    assert_eq!((text[..end].ends_with("3670,\"timestamp\":\"2026-11-14T10:05:02Z\"}\n"), ms), (true, 3670));
    assert_eq!(turn_end(&old, ts("2026-11-14T10:04:00Z")), None);
}

#[test]
fn records_the_pending_turn_once_written() {
    let dir = std::env::temp_dir().join(format!("achaja-transcript-{}", std::process::id()));
    let paths = Paths::at(&dir, &dir);
    std::fs::create_dir_all(paths.state()).ok();
    let file = dir.join("t.jsonl");
    let now_ts = chrono::Utc::now().to_rfc3339();
    let body = format!("{}\n{}\n", line("a", &now_ts, "claude-opus-5-5", 5), closing(&now_ts, 4200));
    std::fs::write(&file, format!("{body}{{\"half")).ok();
    state::write(&paths, TURN, &json!({"at": now() - 5.0}));
    let p = file.to_string_lossy().to_string();
    // the hook itself (without spawning: the test binary is not Achaja.exe)
    state::write(&paths, PENDING, &json!({"path": p, "session": "s", "since": now() - 6.0, "ms": 5000, "effort": "low", "ok": true}));
    record_pending(&paths);
    let e = ledger::read(&paths, 0.0);
    assert_eq!(e.len(), 1);
    assert!(e[0].ms == 4200 && e[0].kind == "session" && e[0].effort == "low" && e[0].session == "s");
    assert_eq!(num(&state::read(&paths, CURSOR), "offset") as usize, body.len());
    assert!(state::read(&paths, PENDING).is_null());
    let _ = std::fs::remove_dir_all(&dir);
}
