//! Finding a logged message on the server again: by its Message-ID in the folder it was moved
//! to, then in the inbox and Achaja's other folders (its UID changes with every move).

use super::accounts::Account;
use super::imap::{quote, Imap};
use super::log::Entry;
use crate::error::{AppError, Result};

pub async fn locate(imap: &mut Imap, a: &Account, entry: &Entry) -> Result<Option<(String, u32)>> {
    let handling = entry.handling.clone().map(|h| h.into_values().collect::<Vec<_>>()).unwrap_or_else(|| a.handling.values().cloned().collect());
    let mut names = vec![if entry.folder.is_empty() { "INBOX".to_string() } else { entry.folder.clone() }, "INBOX".to_string()];
    names.extend(handling.into_iter().filter(|f| f != "keep" && f != "delete"));
    let mut seen = Vec::new();
    for (i, name) in names.iter().enumerate() {
        // the logged folder is already the server's name; the others are Achaja's names
        let folder = if i == 0 { name.clone() } else { imap.server_folder(name).await };
        if seen.contains(&folder) || !imap.exists(&folder).await {
            continue;
        }
        seen.push(folder.clone());
        imap.select(&folder).await?;
        let mut uids = if entry.message_id.is_empty() { Vec::new() } else { imap.search(&format!("HEADER Message-ID {}", quote(&entry.message_id))).await? };
        if uids.is_empty() && folder == entry.folder && entry.uid > 0 {
            uids = imap.search(&format!("UID {}", entry.uid)).await?;
        }
        if let Some(uid) = uids.last() {
            return Ok(Some((folder, *uid)));
        }
    }
    Ok(None)
}

/// (folder, uid, raw message) of a logged entry; the connection stays logged in.
pub async fn fetch_entry(imap: &mut Imap, a: &Account, entry: &Entry) -> Result<(String, u32, Vec<u8>)> {
    let (folder, uid) = locate(imap, a, entry).await?.ok_or_else(|| AppError::msg("Nie znalazlam tej wiadomosci na serwerze (usunieta albo przeniesiona recznie)."))?;
    let fetched = imap.fetch(&[uid], "BODY.PEEK[]").await?;
    let raw = fetched.iter().find_map(|f| f.body().map(<[u8]>::to_vec)).ok_or_else(|| AppError::msg("Pusta wiadomosc."))?;
    Ok((folder, uid, raw))
}
