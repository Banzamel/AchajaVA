//! Backups of Achaja's settings and data (`achaja backup ...`): config.json (mail passwords
//! included!), .mcp.json, the local Claude settings and rules, and state\ without the speech
//! cache, zipped into backup.folder (default <install>\backups), the newest backup.keep kept.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use clap::Subcommand;
use serde_json::json;

use crate::config;
use crate::error::{AppError, Result};
use crate::json::{mtime, now, read_value, write_json};
use crate::paths::Paths;

const FILES: &[&str] = &["config.json", ".mcp.json", ".claude/settings.local.json", ".claude/CLAUDE.local.md"];
const SKIP_DIRS: &[&str] = &["tts_cache", "__pycache__", "scan"];
const SKIP_SUFFIX: &[&str] = &[".mp3", ".wav", ".pid", ".tmp"];

#[derive(Debug, Subcommand)]
pub enum BackupCmd {
    /// Kopia teraz
    Create,
    /// Kopie od najnowszej
    List,
    /// Kopia, jesli ostatnia jest starsza niz backup.every_hours
    Auto,
    /// Przywraca kopie (najpierw kopia obecnego stanu)
    Restore { file: String },
    /// Sciezka folderu kopii
    Folder,
}

pub struct Settings {
    pub folder: PathBuf,
    pub keep: usize,
    pub every_hours: f64,
    pub auto: bool,
}

pub fn settings(paths: &Paths) -> Settings {
    let cfg = config::load_or_default(paths);
    let folder = cfg.str_or("backup.folder", "");
    Settings {
        folder: if folder.is_empty() { paths.install.join("backups") } else { PathBuf::from(folder) },
        keep: cfg.f64_or("backup.keep", 10.0) as usize,
        every_hours: cfg.f64_or("backup.every_hours", 24.0),
        auto: cfg.bool_or("backup.auto", true),
    }
}

fn members(paths: &Paths) -> Vec<(PathBuf, String)> {
    let mut out: Vec<(PathBuf, String)> = FILES.iter().map(|r| (paths.data.join(r), r.to_string())).filter(|(p, _)| p.is_file()).collect();
    fn walk(dir: &Path, rel: &str, out: &mut Vec<(PathBuf, String)>) {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let r = format!("{rel}/{name}");
            if e.path().is_dir() {
                if !SKIP_DIRS.contains(&name.as_str()) {
                    walk(&e.path(), &r, out);
                }
            } else if !SKIP_SUFFIX.iter().any(|s| name.ends_with(s)) {
                out.push((e.path(), r));
            }
        }
    }
    walk(&paths.state(), "state", &mut out);
    out
}

pub fn backups(paths: &Paths) -> Vec<PathBuf> {
    let mut list: Vec<PathBuf> = std::fs::read_dir(settings(paths).folder)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().map(|n| n.to_string_lossy()).is_some_and(|n| n.starts_with("Achaja-") && n.ends_with(".zip")))
        .collect();
    list.sort_by(|a, b| mtime(b).total_cmp(&mtime(a)));
    list
}

pub fn create(paths: &Paths, note: &str) -> Result<(PathBuf, usize)> {
    let s = settings(paths);
    std::fs::create_dir_all(&s.folder)?;
    let suffix = if note.is_empty() { String::new() } else { format!("-{note}") };
    let name = s.folder.join(format!("Achaja-{}{suffix}.zip", chrono::Local::now().format("%Y%m%d-%H%M%S")));
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&name)?);
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut count = 0;
    for (path, rel) in members(paths) {
        let Ok(bytes) = std::fs::read(&path) else { continue }; // a file locked by another process
        zip.start_file(rel, opts)?;
        zip.write_all(&bytes)?;
        count += 1;
    }
    zip.finish()?;
    write_json(&paths.state_file("backup.json"), &json!({"at": now(), "file": name.to_string_lossy()}))?;
    for old in backups(paths).into_iter().skip(s.keep) {
        let _ = std::fs::remove_file(old);
    }
    Ok((name, count))
}

pub fn due(paths: &Paths) -> bool {
    let s = settings(paths);
    s.auto && now() - read_value(&paths.state_file("backup.json"))["at"].as_f64().unwrap_or(0.0) >= s.every_hours * 3600.0
}

pub fn restore(paths: &Paths, file: &str) -> Result<(PathBuf, PathBuf)> {
    let given = PathBuf::from(file);
    let path = if given.is_file() {
        given
    } else {
        let stem = given.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        backups(paths).into_iter().find(|b| b.file_stem().map(|s| s.to_string_lossy() == stem).unwrap_or(false)).ok_or_else(|| AppError::msg(format!("Nie ma kopii {file}.")))?
    };
    let (safety, _) = create(paths, "przed-przywroceniem")?;
    let mut zip = zip::ZipArchive::new(std::fs::File::open(&path)?)?;
    for i in 0..zip.len() {
        let mut f = zip.by_index(i)?;
        let name = f.name().replace('\\', "/");
        // a zip with "..\" never leaves the data folder; only Achaja's own files come back
        if name.split('/').any(|p| p == ".." || p.contains(':')) || !(FILES.contains(&name.as_str()) || name.starts_with("state/")) {
            continue;
        }
        let target = paths.data.join(&name);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut bytes = Vec::new();
        f.read_to_end(&mut bytes)?;
        std::fs::write(target, bytes)?;
    }
    Ok((path, safety))
}

pub fn run(paths: &Paths, cmd: BackupCmd) -> Result<i32> {
    let name = |p: &Path| p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    match cmd {
        BackupCmd::Create => {
            let (n, c) = create(paths, "")?;
            println!("Kopia zapisana: {} ({c} plikow).", n.display());
        }
        BackupCmd::Auto if due(paths) => {
            let (n, c) = create(paths, "")?;
            println!("Kopia zapisana: {} ({c} plikow).", n.display());
        }
        BackupCmd::Auto => println!("Ostatnia kopia jest aktualna."),
        BackupCmd::List => {
            let list = backups(paths);
            if list.is_empty() {
                println!("Brak kopii.");
            }
            for b in list {
                let size = std::fs::metadata(&b).map(|m| m.len() / 1024).unwrap_or(0);
                println!("{}  {size} KB  {}", crate::plan::schedule::from_ts(mtime(&b)).format("%d.%m.%Y %H:%M"), name(&b));
            }
        }
        BackupCmd::Folder => println!("{}", settings(paths).folder.display()),
        BackupCmd::Restore { file } => {
            let (p, s) = restore(paths, &file)?;
            println!("Przywrocono {}. Poprzedni stan zapisalam w {}. Uruchom Achaje ponownie.", name(&p), name(&s));
        }
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_and_restores() {
        let dir = tempfile::tempdir().unwrap();
        let p = Paths::at(&dir.path().join("data"), dir.path());
        std::fs::create_dir_all(p.state().join("tts_cache")).unwrap();
        std::fs::write(p.data.join("config.json"), "{\"backup\": {\"folder\": \"\"}}").unwrap();
        std::fs::write(p.state_file("plan.json"), "{}").unwrap();
        std::fs::write(p.state().join("tts_cache").join("a.mp3"), "x").unwrap();
        let (zip, count) = create(&p, "").unwrap();
        assert_eq!(count, 2);
        std::fs::write(p.state_file("plan.json"), "changed").unwrap();
        restore(&p, &zip.to_string_lossy()).unwrap();
        assert_eq!(std::fs::read_to_string(p.state_file("plan.json")).unwrap(), "{}");
    }
}
