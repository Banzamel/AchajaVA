//! Folder operations of an IMAP connection: the special folders (Sent, Drafts), moving or
//! deleting messages of the selected folder, flags and expunging.

use futures_util::TryStreamExt;

use super::imap::{err, quote, uid_set, Imap};
use super::utf7;
use crate::error::Result;

impl Imap {
    /// \Sent or \Drafts by RFC 6154 flags, else by usual names, else a new folder.
    pub async fn special_folder(&mut self, flag: &str, names: &[&str]) -> Result<String> {
        let list = self.folders().await?;
        if let Some((n, _)) = list.iter().find(|(_, attrs)| attrs.contains(flag)) {
            return Ok(n.clone());
        }
        for name in names {
            for cand in [name.to_string(), self.server_folder(name).await] {
                if let Some((n, _)) = list.iter().find(|(n, _)| utf7::decode(n).eq_ignore_ascii_case(&utf7::decode(&cand))) {
                    return Ok(n.clone());
                }
            }
        }
        self.ensure_folder(names[0]).await
    }

    pub async fn expunge(&mut self, uids: &[u32]) -> Result<()> {
        if self.has("UIDPLUS") {
            let _: Vec<_> = self.session.uid_expunge(uid_set(uids)).await.map_err(err)?.try_collect().await.map_err(err)?;
        } else {
            let _: Vec<_> = self.session.expunge().await.map_err(err)?.try_collect().await.map_err(err)?;
        }
        Ok(())
    }

    pub async fn flag(&mut self, uids: &[u32], flags: &str) -> Result<()> {
        let _: Vec<_> = self.session.uid_store(uid_set(uids), format!("+FLAGS ({flags})")).await.map_err(err)?.try_collect().await.map_err(err)?;
        Ok(())
    }

    /// Moves (or for "delete" removes) messages of the selected folder; the folder they went to.
    pub async fn move_to(&mut self, uids: &[u32], target: &str) -> Result<Option<String>> {
        if uids.is_empty() || target == "keep" {
            return Ok(None);
        }
        if target == "delete" {
            self.flag(uids, "\\Deleted").await?;
            self.expunge(uids).await?;
            return Ok(None);
        }
        let folder = self.ensure_folder(target).await?;
        if self.has("MOVE") {
            self.session.uid_mv(uid_set(uids), quote(&folder)).await.map_err(err)?;
        } else {
            self.session.uid_copy(uid_set(uids), quote(&folder)).await.map_err(err)?;
            self.flag(uids, "\\Deleted").await?;
            self.expunge(uids).await?;
        }
        Ok(Some(folder))
    }

}
