//! Verdicts without a model: the server's antivirus and antispam headers, executable or macro
//! attachments (also inside zip files, and encrypted archives), Windows Defender, and the
//! user's important and ignored senders.

use std::io::Cursor;
use std::path::PathBuf;

use crate::mail::accounts::Account;
use crate::mail::neutral::sender_matches;
use crate::mail::parse::{Attachment, Mail};
use crate::paths::Paths;

const MAX_SCAN_BYTES: usize = 50 * 1024 * 1024;

pub struct Verdict {
    pub category: &'static str,
    pub reason: String,
    /// The model still writes a summary (the user hears about this one).
    pub summarize: bool,
}

pub fn blocked_attachment(att: &Attachment, blocked: &[String]) -> Option<String> {
    let mut names = vec![att.name.clone()];
    let mut encrypted = false;
    if att.name.to_lowercase().ends_with(".zip") && !att.data.is_empty() {
        if let Ok(mut z) = zip::ZipArchive::new(Cursor::new(&att.data)) {
            for i in 0..z.len() {
                if let Ok(f) = z.by_index_raw(i) {
                    names.push(f.name().to_string());
                    encrypted |= f.encrypted();
                }
            }
        }
    }
    for n in &names {
        let ext = n.rsplit_once('.').map(|(_, e)| e.to_lowercase()).unwrap_or_default();
        if !ext.is_empty() && blocked.iter().any(|b| b == &ext) {
            return Some(format!("niebezpieczny zalacznik: {n}"));
        }
    }
    encrypted.then(|| "zaszyfrowane archiwum w zalaczniku".to_string())
}

fn defender() -> PathBuf {
    let pf = std::env::var_os("ProgramFiles").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Program Files"));
    pf.join("Windows Defender").join("MpCmdRun.exe")
}

/// Scans the attachments with Windows Defender; a description of the threat or None.
pub fn defender_scan(paths: &Paths, atts: &[Attachment]) -> Option<String> {
    let exe = defender();
    if !exe.exists() || atts.is_empty() || atts.iter().map(|a| a.size).sum::<usize>() > MAX_SCAN_BYTES {
        return None;
    }
    let folder = paths.state().join("mail").join("scan").join(format!("{}-{}", std::process::id(), uuid::Uuid::new_v4().simple()));
    std::fs::create_dir_all(&folder).ok()?;
    let result = (|| {
        let mut files = Vec::new();
        for (i, a) in atts.iter().enumerate() {
            let ext: String = a.name.rsplit_once('.').map(|(_, e)| e.chars().take(10).collect()).unwrap_or_else(|| "bin".into());
            let path = folder.join(format!("{i}.{ext}"));
            if std::fs::write(&path, &a.data).is_err() {
                return Some(format!("Windows Defender zablokowal zalacznik {}", a.name));
            }
            files.push((a.name.clone(), path));
        }
        std::thread::sleep(std::time::Duration::from_millis(1500)); // real-time protection removes threats at once
        if let Some((name, _)) = files.iter().find(|(_, p)| !p.exists()) {
            return Some(format!("Windows Defender usunal zalacznik {name}"));
        }
        let out = crate::process::hidden(&exe).args(["-Scan", "-ScanType", "3", "-File"]).arg(&folder).arg("-DisableRemediation").output().ok()?;
        (out.status.code() == Some(2)).then(|| "Windows Defender wykryl zagrozenie w zalaczniku".to_string())
    })();
    let _ = std::fs::remove_dir_all(&folder);
    result
}

pub fn verdict(paths: &Paths, m: &Mail, a: &Account) -> Option<Verdict> {
    let v = |category, reason: &str, summarize| Some(Verdict { category, reason: reason.to_string(), summarize });
    if m.server_virus {
        return v("threat", "antywirus serwera", false);
    }
    let blocked = a.list("blocked_extensions");
    if let Some(reason) = m.attachments.iter().find_map(|att| blocked_attachment(att, &blocked)) {
        return v("threat", &reason, true);
    }
    if a.b("scan_attachments", true) && !m.attachments.is_empty() {
        if let Some(reason) = defender_scan(paths, &m.attachments) {
            return v("threat", &reason, true);
        }
    }
    let spoofed = m.auth.contains("dmarc=fail");
    let important = sender_matches(&m.from_addr, &a.list("important_senders"));
    if important && !spoofed {
        return v("important", "nadawca z listy waznych", true);
    }
    if sender_matches(&m.from_addr, &a.list("ignored_senders")) {
        return v("normal", "nadawca z listy pomijanych", false);
    }
    if m.server_spam && !important {
        return v("spam", "antyspam serwera", false);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_executables_and_macros() {
        let att = |name: &str| Attachment { name: name.into(), size: 1, data: vec![1] };
        let blocked = vec!["exe".to_string(), "docm".to_string()];
        assert!(blocked_attachment(&att("faktura.pdf.exe"), &blocked).is_some());
        assert!(blocked_attachment(&att("umowa.DOCM"), &blocked).is_some());
        assert!(blocked_attachment(&att("zdjecie.jpg"), &blocked).is_none());
    }
}
