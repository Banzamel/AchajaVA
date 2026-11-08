//! Writing and sending: a reply or a new mail (with the account's signature and the quoted
//! original), sent over SMTP of the same server by default, with a copy in Sent (or saved as a
//! draft in Drafts) and the original marked as answered.

use std::time::Duration;

use lettre::message::header::ContentType;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use super::accounts::Account;
use super::imap::{self, Imap};
use crate::error::{AppError, Result};

pub struct Draft {
    pub to: String,
    pub cc: Option<String>,
    pub subject: String,
    pub text: String,
    pub in_reply_to: Option<String>,
    pub references: Option<String>,
    pub quote: Option<String>,
}

fn mailboxes(list: &str) -> Result<Vec<Mailbox>> {
    split_addresses(list).iter().map(|a| a.parse::<Mailbox>().map_err(|e| AppError::msg(format!("Zly adres '{a}': {e}")))).collect()
}

/// "a@x, \"B, C\" <b@y>" -> the addresses (commas inside quotes stay).
pub fn split_addresses(list: &str) -> Vec<String> {
    let (mut out, mut cur, mut quoted) = (Vec::new(), String::new(), false);
    for c in list.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                cur.push(c);
            }
            ',' if !quoted => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out.into_iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

pub fn compose(a: &Account, d: &Draft) -> Result<Message> {
    let from = format!("{} <{}>", a.s("display_name"), a.address).trim().parse::<Mailbox>().map_err(|e| AppError::msg(e.to_string()))?;
    let domain = a.address.rsplit('@').next().unwrap_or("localhost");
    let mut b = Message::builder().from(from).subject(&d.subject).date_now().message_id(Some(format!("<{}@{domain}>", uuid::Uuid::new_v4().simple())));
    for m in mailboxes(&d.to)? {
        b = b.to(m);
    }
    for m in mailboxes(d.cc.as_deref().unwrap_or(""))? {
        b = b.cc(m);
    }
    if let Some(id) = &d.in_reply_to {
        b = b.in_reply_to(id.clone());
        b = b.references(d.references.as_ref().map(|r| format!("{r} {id}")).unwrap_or_else(|| id.clone()).trim().to_string());
    }
    let mut body = d.text.trim_end().to_string();
    if !a.s("signature").is_empty() {
        body += &format!("\n\n-- \n{}", a.s("signature"));
    }
    if let Some(q) = &d.quote {
        body += &format!("\n\n{q}");
    }
    b.header(ContentType::TEXT_PLAIN).body(body + "\n").map_err(|e| AppError::msg(e.to_string()))
}

fn transport(a: &Account, timeout: u64) -> Result<(AsyncSmtpTransport<Tokio1Executor>, String, u16, String)> {
    let default_sec = if a.security() == "ssl" { "ssl" } else { "starttls" };
    let security = Some(a.s("smtp_security")).filter(|s| !s.is_empty()).unwrap_or_else(|| default_sec.into());
    let host = Some(a.s("smtp_host")).filter(|s| !s.is_empty()).unwrap_or_else(|| a.host());
    let port = match a.n("smtp_port", 0.0) as u16 {
        0 => match security.as_str() {
            "ssl" => 465,
            "starttls" => 587,
            _ => 25,
        },
        p => p,
    };
    crate::runtime::install_tls();
    let params = TlsParameters::builder(host.clone()).dangerous_accept_invalid_certs(!a.b("verify_tls", true)).build_rustls().map_err(|e| AppError::Mail(e.to_string()))?;
    let tls = match security.as_str() {
        "ssl" => Tls::Wrapper(params),
        "starttls" => Tls::Required(params),
        _ => Tls::None,
    };
    let mut b = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&host).port(port).tls(tls).timeout(Some(Duration::from_secs(timeout)));
    if security != "none" || a.b("smtp_auth", false) {
        let user = Some(a.s("smtp_user")).filter(|s| !s.is_empty()).unwrap_or_else(|| a.user());
        let pass = Some(a.s("smtp_password")).filter(|s| !s.is_empty()).unwrap_or_else(|| a.s("password"));
        b = b.credentials(Credentials::new(user, pass));
    }
    Ok((b.build(), host, port, security))
}

/// Logs in to SMTP without sending: "host:port, security".
pub async fn test(a: &Account) -> Result<String> {
    let (t, host, port, security) = transport(a, 20)?;
    t.test_connection().await.map_err(|e| AppError::Mail(e.to_string()))?;
    Ok(format!("SMTP {host}:{port}, {security}"))
}

/// Sends (or saves as a draft) and keeps a copy on the IMAP server; a description of what happened.
pub async fn deliver(a: &Account, msg: Message, draft: bool, dry_run: bool, imap: Option<&mut Imap>, answered: Option<(String, u32)>) -> Result<String> {
    let raw = msg.formatted();
    if dry_run {
        return Ok(format!("=== PODGLAD (nic nie wyslano) ===\n{}", String::from_utf8_lossy(&raw).chars().take(6000).collect::<String>()));
    }
    if !draft {
        transport(a, 60)?.0.send(msg.clone()).await.map_err(|e| AppError::Mail(e.to_string()))?;
    }
    let mut own = None;
    let conn = match imap {
        Some(c) => Some(c),
        None => {
            own = imap::connect(a).await.ok();
            own.as_mut()
        }
    };
    let copy = match conn {
        Some(c) => store_copy(c, &raw, draft, answered).await.unwrap_or_else(|e| format!("(nie zapisano kopii: {e})")),
        None => "(nie zapisano kopii: brak polaczenia IMAP)".into(),
    };
    if let Some(c) = own {
        c.logout().await;
    }
    let head = |name: &str| msg.headers().get_raw(name).map(str::to_string).unwrap_or_default();
    let cc = Some(head("Cc")).filter(|c| !c.is_empty()).map(|c| format!(", DW: {c}")).unwrap_or_default();
    Ok(format!("{} do: {}{cc} | temat: {} | kopia: {copy}", if draft { "Zapisano szkic" } else { "Wyslano" }, head("To"), head("Subject")))
}

async fn store_copy(c: &mut Imap, raw: &[u8], draft: bool, answered: Option<(String, u32)>) -> Result<String> {
    let (folder, flags) = if draft {
        (c.special_folder("Drafts", &["Drafts", "Szkice", "INBOX.Drafts"]).await?, "(\\Draft \\Seen)")
    } else {
        (c.special_folder("Sent", &["Sent", "Wysłane", "Sent Items", "INBOX.Sent"]).await?, "(\\Seen)")
    };
    c.session.append(imap::quote(&folder), Some(flags), None, raw).await.map_err(imap::err)?;
    if let (Some((f, uid)), false) = (answered, draft) {
        c.select(&f).await?;
        c.flag(&[uid], "\\Answered").await?;
    }
    Ok(folder)
}
