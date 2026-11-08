//! Leaving a mailing list: one-click POST (RFC 8058) only to public addresses and without
//! following redirects; a mailto goes back to Claude to send after the user's yes; a web link is
//! only named, never opened. Every attempt is remembered per sender.

use std::net::{IpAddr, ToSocketAddrs};

use serde_json::{json, Value};

use super::{file, norm};
use crate::json::{read_json, write_json};
use crate::paths::Paths;

/// List-Unsubscribe: <https://...>, <mailto:...> -> (https links, mailto links).
pub fn parse_header(value: &str) -> (Vec<String>, Vec<String>) {
    let mut urls: Vec<String> = value.split('<').skip(1).filter_map(|p| p.split('>').next()).map(|u| u.trim().to_string()).filter(|u| !u.is_empty()).collect();
    if urls.is_empty() {
        urls = value.split(',').map(|u| u.trim().to_string()).filter(|u| !u.is_empty()).collect();
    }
    let https = urls.iter().filter(|u| u.to_lowercase().starts_with("https://")).cloned().collect();
    let mailto = urls.iter().filter(|u| u.to_lowercase().starts_with("mailto:")).cloned().collect();
    (https, mailto)
}

/// Only public addresses: a link from a mail must never reach the home network.
fn public_host(url: &str) -> bool {
    let Ok(u) = reqwest::Url::parse(url) else { return false };
    let Some(host) = u.host_str() else { return false };
    let Ok(addrs) = (host, 443).to_socket_addrs() else { return false };
    let addrs: Vec<IpAddr> = addrs.map(|a| a.ip()).collect();
    !addrs.is_empty()
        && addrs.iter().all(|ip| match ip {
            IpAddr::V4(v) => !(v.is_private() || v.is_loopback() || v.is_link_local() || v.is_multicast() || v.is_unspecified() || v.is_broadcast()),
            IpAddr::V6(v) => !(v.is_loopback() || v.is_multicast() || v.is_unspecified() || (v.segments()[0] & 0xfe00) == 0xfc00 || (v.segments()[0] & 0xffc0) == 0xfe80),
        })
}

async fn one_click(url: &str) -> (bool, String) {
    if !public_host(url) {
        return (false, "adres niepubliczny - pomijam".into());
    }
    crate::runtime::install_tls();
    let Ok(client) = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).timeout(std::time::Duration::from_secs(20)).user_agent("Achaja-unsubscribe/1.0").build() else {
        return (false, "HTTP".into());
    };
    match client.post(url).header("Content-Type", "application/x-www-form-urlencoded").body("List-Unsubscribe=One-Click").send().await {
        Ok(r) => (r.status().is_success(), format!("HTTP {}", r.status().as_u16())),
        Err(e) => (false, format!("blad polaczenia: {e}")),
    }
}

fn remember(paths: &Paths, sender: &str, how: &str, ok: bool) {
    let mut data: Vec<Value> = read_json(&file(paths, "unsubscribed.json"), Vec::new());
    data.insert(0, json!({"sender": norm(sender), "how": how, "ok": ok, "at": chrono::Local::now().format("%Y-%m-%d %H:%M").to_string()}));
    data.truncate(500);
    let _ = write_json(&file(paths, "unsubscribed.json"), &data);
}

fn query_param(url: &str, key: &str, default: &str) -> String {
    reqwest::Url::parse(url).ok().and_then(|u| u.query_pairs().find(|(k, _)| k == key).map(|(_, v)| v.into_owned())).unwrap_or_else(|| default.to_string())
}

pub async fn unsubscribe(paths: &Paths, header: &str, post: &str, sender: &str) -> i32 {
    let (https, mailto) = parse_header(header);
    let sender = if sender.is_empty() { "?" } else { sender };
    let done: Vec<Value> = read_json(&file(paths, "unsubscribed.json"), Vec::new());
    if let Some(d) = done.iter().find(|u| u["ok"] == json!(true) && u["sender"] == json!(norm(sender)) && sender != "?") {
        println!("JUZ WYPISANO {} ({}) - nadawca nadal wysyla; oznacz jako spam.", d["at"].as_str().unwrap_or(""), d["how"].as_str().unwrap_or(""));
        return 0;
    }
    if !https.is_empty() && post.to_lowercase().contains("one-click") {
        for url in &https {
            let (ok, info) = one_click(url).await;
            if ok {
                remember(paths, sender, "one-click", true);
                println!("WYPISANO (one-click, {info}).");
                return 0;
            }
            println!("one-click nie zadzialal: {info}");
        }
    }
    if let Some(m) = mailto.first() {
        remember(paths, sender, "mailto", false);
        let to = m[7..].split('?').next().unwrap_or("");
        let q = format!("x://y/?{}", m.split_once('?').map(|x| x.1).unwrap_or(""));
        println!("MAILTO: do={to} temat={} tresc={}", query_param(&q, "subject", "unsubscribe"), query_param(&q, "body", "unsubscribe"));
        println!("Wyslij to narzedziem send_message konektora Gmail (bramka zapyta o zgode), potem: gmail log.");
        return 3;
    }
    if let Some(link) = https.first() {
        remember(paths, sender, "link", false);
        let host = reqwest::Url::parse(link).ok().and_then(|u| u.host_str().map(str::to_string)).unwrap_or_default();
        println!("RECZNIE: nadawca ma tylko link do strony wypisania ({host}). Nie otwieraj go sama - powiedz uzytkownikowi albo oznacz jako spam.");
        return 3;
    }
    println!("BRAK naglowka List-Unsubscribe - nie da sie wypisac automatycznie; oznacz jako spam albo dodaj junk.");
    3
}

