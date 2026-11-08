//! A raw e-mail turned into what the triage and the commands need: sender, reply address,
//! subject, threading headers, the server's spam and virus verdicts, authentication results,
//! a plain-text body and the attachments.

use std::sync::OnceLock;

use mail_parser::{Address, HeaderValue, MessageParser, MimeHeaders};
use regex::Regex;

#[derive(Debug, Clone)]
pub struct Attachment {
    pub name: String,
    pub size: usize,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
pub struct Mail {
    pub from_name: String,
    pub from_addr: String,
    pub reply_to: String,
    pub to: String,
    pub cc: String,
    pub subject: String,
    pub date: String,
    pub message_id: String,
    pub references: String,
    pub auth: String,
    pub newsletter: bool,
    pub server_spam: bool,
    pub server_virus: bool,
    pub text: String,
    pub attachments: Vec<Attachment>,
    pub unread: bool,
}

fn addresses(a: Option<&Address>) -> Vec<(String, String)> {
    a.map(|a| a.iter().map(|x| (x.name().unwrap_or("").to_string(), x.address().unwrap_or("").to_string())).collect()).unwrap_or_default()
}

pub fn format_addr(name: &str, addr: &str) -> String {
    if name.is_empty() {
        addr.to_string()
    } else {
        format!("\"{}\" <{addr}>", name.replace('"', ""))
    }
}

fn joined(list: &[(String, String)]) -> String {
    list.iter().map(|(n, a)| format_addr(n, a)).collect::<Vec<_>>().join(", ")
}

fn header_text(v: &HeaderValue) -> String {
    match v {
        HeaderValue::Text(t) => t.to_string(),
        HeaderValue::TextList(l) => l.iter().map(|s| s.as_ref()).collect::<Vec<_>>().join(" "),
        _ => String::new(),
    }
}

fn re(cell: &'static OnceLock<Option<Regex>>, p: &str) -> Option<&'static Regex> {
    cell.get_or_init(|| Regex::new(p).ok()).as_ref()
}

/// HTML to readable text (scripts and styles dropped, block ends as new lines).
pub fn html_to_text(html: &str) -> String {
    static DROP: OnceLock<Option<Regex>> = OnceLock::new();
    static BREAK: OnceLock<Option<Regex>> = OnceLock::new();
    static TAGS: OnceLock<Option<Regex>> = OnceLock::new();
    let mut t = html.to_string();
    for (cell, p, with) in [
        (&DROP, r"(?is)<(?:script|style|head)\b.*?</(?:script|style|head)>", " "),
        (&BREAK, r"(?i)<br\s*/?>|</p>|</div>|</tr>", "\n"),
        (&TAGS, r"<[^>]+>", " "),
    ] {
        if let Some(r) = re(cell, p) {
            t = r.replace_all(&t, with).into_owned();
        }
    }
    html_escape::decode_html_entities(&t).into_owned()
}

fn tidy(text: &str) -> String {
    static PARAS: OnceLock<Option<Regex>> = OnceLock::new();
    static SPACES: OnceLock<Option<Regex>> = OnceLock::new();
    let t = re(&PARAS, r"\n\s*\n+").map_or(text.to_string(), |r| r.replace_all(text, "\n\n").into_owned());
    let t = re(&SPACES, r"[ \t\r\x0c\x0b]+").map_or(t.clone(), |r| r.replace_all(&t, " ").into_owned());
    t.trim().to_string()
}

pub fn parse(raw: &[u8]) -> Mail {
    let Some(msg) = MessageParser::default().parse(raw) else { return Mail::default() };
    let from = addresses(msg.from());
    let (from_name, from_addr) = from.first().cloned().unwrap_or_default();
    let raw_header = |name: &str| msg.header_raw(name).map(|s| s.trim().to_string()).unwrap_or_default();
    let auth = raw_header("Authentication-Results").to_lowercase();
    static AUTH: OnceLock<Option<Regex>> = OnceLock::new();
    let mut found: Vec<String> = re(&AUTH, r"\b(?:dmarc|spf|dkim)=\w+").map(|r| r.find_iter(&auth).map(|m| m.as_str().to_string()).collect()).unwrap_or_default();
    found.sort();
    found.dedup();
    let spam_flag = raw_header("X-Spam-Flag").to_lowercase();
    let spam_status = raw_header("X-Spam-Status").to_lowercase();
    let spam = raw_header("X-Spam").to_lowercase();
    let virus = format!("{} {}", raw_header("X-Virus-Status"), raw_header("X-Virus")).to_lowercase();
    // a real text/plain part when there is one, else our reading of the HTML
    let plain = msg.text_bodies().find_map(|p| match &p.body {
        mail_parser::PartType::Text(t) => Some(t.to_string()),
        _ => None,
    });
    let text = plain.unwrap_or_else(|| msg.body_html(0).map(|h| html_to_text(&h)).unwrap_or_default());
    let attachments = msg
        .attachments()
        .map(|p| Attachment { name: p.attachment_name().unwrap_or("bez_nazwy").to_string(), size: p.contents().len(), data: p.contents().to_vec() })
        .collect();
    Mail {
        from_name: from_name.trim().trim_matches('"').to_string(),
        from_addr: from_addr.to_lowercase(),
        reply_to: addresses(msg.reply_to()).into_iter().map(|(_, a)| a).filter(|a| !a.is_empty()).collect::<Vec<_>>().join(", "),
        to: joined(&addresses(msg.to())),
        cc: joined(&addresses(msg.cc())),
        subject: crate::text::squash(msg.subject().unwrap_or("")),
        date: msg.date().map(|d| d.to_rfc822()).unwrap_or_default(),
        message_id: msg.message_id().map(|m| format!("<{}>", m.trim_matches(['<', '>']))).unwrap_or_default(),
        references: crate::text::squash(&header_text(msg.references())),
        auth: found.join(" "),
        newsletter: !raw_header("List-Unsubscribe").is_empty() || !raw_header("List-Id").is_empty(),
        server_spam: spam_flag == "yes" || spam_status.starts_with("yes") || spam == "yes" || spam == "true",
        server_virus: virus.contains("infected") || virus.contains("virus found"),
        text: tidy(&text),
        attachments,
        unread: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_headers_body_and_verdicts() {
        let raw = b"From: \"Jan K\" <Jan@Firma.pl>\r\nReply-To: other@x.com\r\nTo: me@a.pl\r\nSubject: Faktura\r\n  nr 5\r\nMessage-ID: <abc@firma.pl>\r\nX-Spam-Status: Yes, score=9\r\nAuthentication-Results: mx; dmarc=fail; spf=pass\r\nList-Unsubscribe: <mailto:u@x>\r\nContent-Type: text/html\r\n\r\n<p>Hello&amp;bye</p><script>x</script>";
        let m = parse(raw);
        assert_eq!(m.from_name, "Jan K");
        assert_eq!(m.from_addr, "jan@firma.pl");
        assert_eq!(m.reply_to, "other@x.com");
        assert_eq!(m.subject, "Faktura nr 5");
        assert_eq!(m.message_id, "<abc@firma.pl>");
        assert!(m.server_spam && m.newsletter && !m.server_virus);
        assert_eq!(m.auth, "dmarc=fail spf=pass");
        assert_eq!(m.text, "Hello&bye");
    }
}
