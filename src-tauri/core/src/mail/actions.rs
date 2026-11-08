//! The mail actions on one logged message: reading it, moving it (a correction, or back to the
//! inbox) and replying to it.

use super::accounts::{find_account, lang, CATEGORIES};
use super::log::{self, USER_FIX};
use super::neutral::{fenced, neutral};
use super::parse::{format_addr, parse};
use super::report::label;
use super::{find, imap, smtp};
use crate::config::Config;
use crate::error::{AppError, Result};
use crate::paths::Paths;

pub fn read_body(file: &str) -> Result<String> {
    let text = crate::json::read_text(std::path::Path::new(file)).unwrap_or_default().trim().to_string();
    if text.is_empty() {
        return Err(AppError::msg("Plik z trescia jest pusty."));
    }
    Ok(text)
}

pub async fn show(paths: &Paths, cfg: &Config, id: u64) -> Result<()> {
    let entry = log::get(paths, id)?;
    let a = find_account(cfg, &entry.account)?;
    let mut c = imap::connect(&a).await?;
    let (folder, _, raw) = find::fetch_entry(&mut c, &a, &entry).await?;
    c.logout().await;
    let m = parse(&raw);
    let mut lines = vec![
        format!("Od: {} <{}>{}", m.from_name, m.from_addr, if m.reply_to.is_empty() { String::new() } else { format!(" | Odpowiedz do: {}", m.reply_to) }),
        format!("Do: {}{}", m.to, if m.cc.is_empty() { String::new() } else { format!(" | DW: {}", m.cc) }),
        format!("Data: {}", m.date),
        format!("Temat: {}", m.subject),
    ];
    if !m.attachments.is_empty() {
        lines.push(format!("Zalaczniki: {}", m.attachments.iter().map(|x| format!("{} ({} KB)", x.name, x.size / 1024)).collect::<Vec<_>>().join(", ")));
    }
    let lines: Vec<String> = lines.iter().map(|l| neutral(l, true)).collect();
    println!("Folder: {folder}");
    if !m.auth.is_empty() {
        println!("Uwierzytelnienie: {}", m.auth);
    }
    let body = if m.text.is_empty() { "(brak tresci tekstowej)".to_string() } else { crate::text::truncate_chars(&m.text, 8000) };
    println!("{}", fenced(&format!("{}\n\n{body}", lines.join("\n"))));
    Ok(())
}

pub async fn move_entry(paths: &Paths, cfg: &Config, id: u64, target: &str) -> Result<()> {
    let entry = log::get(paths, id)?;
    let a = find_account(cfg, &entry.account)?;
    let t = target.to_lowercase();
    let (dest_name, category) = if t == "inbox" {
        (a.folder(), "normal".to_string())
    } else if CATEGORIES.contains(&t.as_str()) {
        let h = a.handling.get(&t).cloned().unwrap_or_else(|| "keep".into());
        (if h == "keep" { a.folder() } else { h }, t.clone())
    } else {
        return Err(AppError::msg(format!("Nieznany cel '{target}'. Dostepne: inbox, {}.", CATEGORIES.join(", "))));
    };
    let mut c = imap::connect(&a).await?;
    let found = find::locate(&mut c, &a, &entry).await?;
    let Some((folder, uid)) = found else {
        c.logout().await;
        return Err(AppError::msg("Nie znalazlam tej wiadomosci na serwerze (juz usunieta)."));
    };
    let dest = if dest_name == "delete" { "usunieta".to_string() } else { c.server_folder(&dest_name).await };
    if dest != folder {
        c.move_to(&[uid], &dest_name).await?;
    }
    c.logout().await;
    let cat = category.clone();
    let d = dest.clone();
    log::update(paths, entry.id, move |e| {
        e.category = cat.clone();
        e.folder = d.clone();
        e.reason = USER_FIX.into();
        e.user_fixed = true;
    });
    println!("Wiadomosc [{}] -> {dest} (kategoria: {}).", entry.id, label(&category, lang(cfg)));
    Ok(())
}

pub async fn reply(paths: &Paths, cfg: &Config, id: u64, text_file: &str, all: bool, draft: bool, dry_run: bool) -> Result<()> {
    let entry = log::get(paths, id)?;
    let a = find_account(cfg, &entry.account)?;
    let text = read_body(text_file)?;
    let mut c = imap::connect(&a).await?;
    let (folder, uid, raw) = find::fetch_entry(&mut c, &a, &entry).await?;
    let m = parse(&raw);
    let to = if m.reply_to.is_empty() { format_addr(&m.from_name, &m.from_addr) } else { m.reply_to.clone() };
    let cc = all.then(|| {
        let me = a.address.to_lowercase();
        let others: Vec<String> = smtp::split_addresses(&format!("{}, {}", m.to, m.cc))
            .into_iter()
            .filter(|x| {
                let addr = x.rsplit('<').next().unwrap_or(x).trim_end_matches('>').to_lowercase();
                addr != me && addr != m.from_addr
            })
            .collect();
        others.join(", ")
    }).filter(|c| !c.is_empty());
    let low = m.subject.to_lowercase();
    let subject = if low.starts_with("re:") || low.starts_with("odp:") || low.starts_with("aw:") { m.subject.clone() } else { format!("Re: {}", m.subject) };
    let wrote = if lang(cfg) == "pl" { "napisał(a)" } else { "wrote" };
    let quoted: Vec<String> = m.text.lines().take(80).map(|l| format!("> {l}")).collect();
    let who = if m.from_name.is_empty() { &m.from_addr } else { &m.from_name };
    let draft_msg = smtp::Draft { to, cc, subject, text, in_reply_to: Some(m.message_id.clone()).filter(|s| !s.is_empty()), references: Some(m.references.clone()).filter(|s| !s.is_empty()), quote: Some(format!("{}, {who} {wrote}:\n{}", m.date, quoted.join("\n"))) };
    let msg = smtp::compose(&a, &draft_msg)?;
    let out = smtp::deliver(&a, msg, draft, dry_run, Some(&mut c), Some((folder, uid))).await;
    c.logout().await;
    println!("{}", out?);
    Ok(())
}
