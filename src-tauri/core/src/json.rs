//! JSON and text files of the data folder. Writes go to a temporary file first and are renamed
//! over the target, so a crash never leaves half a settings or state file. Reads never fail:
//! a missing or broken file gives the default, like the state files always have.

use std::io::Write;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use crate::error::Result;

pub fn read_json<T: DeserializeOwned>(path: &Path, default: T) -> T {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(strip_bom(&bytes)).ok())
        .unwrap_or(default)
}

pub fn read_value(path: &Path) -> Value {
    read_json(path, Value::Null)
}

/// A BOM from Windows editors (Notepad) must not break config.json.
pub fn strip_bom(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes)
}

pub fn write_json<T: Serialize + ?Sized>(path: &Path, data: &T) -> Result<()> {
    let text = serde_json::to_string_pretty(data)?;
    write_text(path, &text)
}

pub fn write_text(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(|e| e.to_str()).unwrap_or("")
    ));
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

pub fn read_text(path: &Path) -> Option<String> {
    std::fs::read(path).ok().map(|b| String::from_utf8_lossy(strip_bom(&b)).into_owned())
}

pub fn append_line(path: &Path, line: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}

pub fn remove(path: &Path) {
    let _ = std::fs::remove_file(path);
}

pub fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

pub fn mtime(path: &Path) -> f64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// A field of a JSON object read as a number (state files store times as floats).
pub fn num(value: &Value, key: &str) -> f64 {
    value.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}
