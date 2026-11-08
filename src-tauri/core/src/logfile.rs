//! The Log tab's lines also go to data\logs\achaja.log (dated), so a failure can be read without
//! the window. At 1 MB the file becomes achaja.1.log (the one before is dropped).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const MAX_BYTES: u64 = 1024 * 1024;

static LOCK: Mutex<()> = Mutex::new(());

pub fn file(logs: &Path) -> PathBuf {
    logs.join("achaja.log")
}

/// Appends one line ("21:33:39 text" from the feed gets the date in front).
pub fn append(logs: &Path, line: &str) {
    let _guard = LOCK.lock();
    let path = file(logs);
    if std::fs::metadata(&path).map(|m| m.len() >= MAX_BYTES).unwrap_or(false) {
        let _ = std::fs::rename(&path, logs.join("achaja.1.log"));
    }
    if std::fs::create_dir_all(logs).is_err() {
        return;
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "{} {line}", chrono::Local::now().format("%Y-%m-%d"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotates_at_the_limit() {
        let dir = tempfile::tempdir().unwrap();
        append(dir.path(), "12:00:00 first");
        assert!(std::fs::read_to_string(file(dir.path())).unwrap().ends_with("12:00:00 first\n"));
        std::fs::write(file(dir.path()), vec![b'x'; MAX_BYTES as usize]).unwrap();
        append(dir.path(), "12:00:01 second");
        assert!(dir.path().join("achaja.1.log").exists());
        let now = std::fs::read_to_string(file(dir.path())).unwrap();
        assert_eq!(now.lines().count(), 1);
    }
}
