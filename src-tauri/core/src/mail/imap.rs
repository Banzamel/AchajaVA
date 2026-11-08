//! An IMAP connection of one account (SSL or STARTTLS, optionally without certificate checks for
//! a self-signed home server) and the folder operations of the mail code: server folder names,
//! special folders, moving, deleting and finding a message again by its Message-ID.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use async_imap::types::Fetch;
use futures_util::TryStreamExt;
use rustls::pki_types::ServerName;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;

use super::accounts::Account;
use super::utf7;
use crate::error::{AppError, Result};

pub trait Stream: AsyncRead + AsyncWrite + Unpin + Send + std::fmt::Debug {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send + std::fmt::Debug> Stream for T {}
pub type Session = async_imap::Session<Box<dyn Stream>>;

pub fn err(e: impl std::fmt::Display) -> AppError {
    AppError::Mail(e.to_string())
}

pub fn tls_config(verify: bool) -> Arc<rustls::ClientConfig> {
    crate::runtime::install_tls();
    if !verify {
        return crate::cast::insecure_config();
    }
    let roots = rustls::RootCertStore { roots: webpki_roots::TLS_SERVER_ROOTS.to_vec() };
    Arc::new(rustls::ClientConfig::builder().with_root_certificates(roots).with_no_client_auth())
}

async fn tls(host: &str, verify: bool, tcp: TcpStream) -> Result<tokio_rustls::client::TlsStream<TcpStream>> {
    let name = ServerName::try_from(host.to_string()).map_err(err)?;
    tokio_rustls::TlsConnector::from(tls_config(verify)).connect(name, tcp).await.map_err(err)
}

pub struct Imap {
    pub session: Session,
    pub caps: HashSet<String>,
    ns: Option<(String, String)>,
}

pub async fn connect(a: &Account) -> Result<Imap> {
    let (host, verify) = (a.host(), a.b("verify_tls", true));
    let tcp = tokio::time::timeout(Duration::from_secs(30), TcpStream::connect((host.as_str(), a.port())))
        .await
        .map_err(|_| err(format!("{host}: timeout")))?
        .map_err(err)?;
    let stream: Box<dyn Stream> = if a.security() == "ssl" {
        let mut client = async_imap::Client::new(tls(&host, verify, tcp).await?);
        client.read_response().await.map_err(err)?; // greeting
        Box::new(client.into_inner())
    } else {
        let mut client = async_imap::Client::new(tcp);
        client.read_response().await.map_err(err)?;
        if a.security() == "starttls" {
            client.run_command_and_check_ok("STARTTLS", None).await.map_err(err)?;
            Box::new(tls(&host, verify, client.into_inner()).await?)
        } else {
            Box::new(client.into_inner())
        }
    };
    let client = async_imap::Client::new(stream);
    let mut session = client.login(a.user(), a.s("password")).await.map_err(|(e, _)| err(format!("login: {e}")))?;
    let caps = session.capabilities().await.map_err(err)?.iter().map(|c| format!("{c:?}").to_uppercase()).collect();
    Ok(Imap { session, caps, ns: None })
}

pub fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

impl Imap {
    pub fn has(&self, cap: &str) -> bool {
        self.caps.iter().any(|c| c.contains(cap))
    }

    async fn namespace(&mut self) -> (String, String) {
        if let Some(ns) = &self.ns {
            return ns.clone();
        }
        let mut found = (String::new(), "/".to_string());
        if let Ok(mut names) = self.session.list(Some(""), Some("\"\"")).await {
            if let Ok(Some(n)) = names.try_next().await {
                found.1 = n.delimiter().unwrap_or("/").to_string();
            }
        }
        if let Ok(id) = self.session.run_command("NAMESPACE").await {
            while let Ok(Some(resp)) = self.session.read_response().await {
                let raw = String::from_utf8_lossy(resp.borrow_owner()).to_string();
                if let Some(c) = regex::Regex::new(r#"^\* NAMESPACE \(\("([^"]*)" (?:"([^"]*)"|NIL)\)"#).ok().and_then(|r| r.captures(&raw)) {
                    found = (c[1].to_string(), c.get(2).map_or(found.1.clone(), |d| d.as_str().to_string()));
                }
                if resp.request_id() == Some(&id) {
                    break;
                }
            }
        }
        self.ns = Some(found.clone());
        found
    }

    /// "Achaja/Reklamy" -> the server's form (separator, personal prefix, UTF-7).
    pub async fn server_folder(&mut self, name: &str) -> String {
        if name.eq_ignore_ascii_case("INBOX") {
            return "INBOX".into();
        }
        let (prefix, delim) = self.namespace().await;
        let path = utf7::encode(&name.replace('/', &delim));
        if prefix.is_empty() || path.starts_with(&prefix) {
            path
        } else {
            format!("{prefix}{path}")
        }
    }

    pub async fn folders(&mut self) -> Result<Vec<(String, String)>> {
        let names: Vec<_> = self.session.list(Some(""), Some("\"*\"")).await.map_err(err)?.try_collect().await.map_err(err)?;
        Ok(names.iter().map(|n| (n.name().to_string(), n.attributes().iter().map(|a| format!("{a:?}")).collect::<Vec<_>>().join(" "))).collect())
    }

    pub async fn exists(&mut self, folder: &str) -> bool {
        match self.session.list(Some(""), Some(&quote(folder))).await {
            Ok(stream) => stream.try_collect::<Vec<_>>().await.map(|v| !v.is_empty()).unwrap_or(false),
            Err(_) => false,
        }
    }

    pub async fn ensure_folder(&mut self, name: &str) -> Result<String> {
        let folder = self.server_folder(name).await;
        if !self.exists(&folder).await {
            self.session.create(quote(&folder)).await.map_err(err)?;
            let _ = self.session.subscribe(quote(&folder)).await;
        }
        Ok(folder)
    }

    pub async fn select(&mut self, folder: &str) -> Result<async_imap::types::Mailbox> {
        self.session.select(quote(folder)).await.map_err(err)
    }

    pub async fn search(&mut self, query: &str) -> Result<Vec<u32>> {
        let mut uids: Vec<u32> = self.session.uid_search(query).await.map_err(err)?.into_iter().collect();
        uids.sort_unstable();
        Ok(uids)
    }

    pub async fn fetch(&mut self, uids: &[u32], items: &str) -> Result<Vec<Fetch>> {
        if uids.is_empty() {
            return Ok(Vec::new());
        }
        let set = uid_set(uids);
        self.session.uid_fetch(set, items).await.map_err(err)?.try_collect().await.map_err(err)
    }

    /// IDLE until new mail, `renew` passes, or `wake()` asks to stop waiting (checked every 30 s).
    pub async fn idle(self, renew: Duration, wake: impl Fn() -> bool) -> Result<Imap> {
        let Imap { session, caps, ns } = self;
        let mut handle = session.idle();
        handle.init().await.map_err(err)?;
        let start = std::time::Instant::now();
        while start.elapsed() < renew {
            let (fut, _stop) = handle.wait_with_timeout(Duration::from_secs(30));
            if let async_imap::extensions::idle::IdleResponse::NewData(_) = fut.await.map_err(err)? {
                break;
            }
            if wake() {
                break;
            }
        }
        let session = handle.done().await.map_err(err)?;
        Ok(Imap { session, caps, ns })
    }

    pub async fn logout(mut self) {
        let _ = self.session.logout().await;
    }
}

pub fn uid_set(uids: &[u32]) -> String {
    uids.iter().map(u32::to_string).collect::<Vec<_>>().join(",")
}

/// IMAP date for SINCE / BEFORE ("06-Nov-2026").
pub fn imap_date(d: chrono::NaiveDate) -> String {
    d.format("%d-%b-%Y").to_string()
}

pub fn flags_of(f: &Fetch) -> Vec<String> {
    f.flags().map(|fl| format!("{fl:?}")).collect()
}
