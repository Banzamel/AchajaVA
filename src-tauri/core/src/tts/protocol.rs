//! The Edge read-aloud websocket protocol (as edge-tts speaks it): the text messages we send
//! (speech config, SSML) and the binary audio frames we receive.

pub const CHROMIUM_FULL_VERSION: &str = "143.0.3650.75";
pub const WSS_URL: &str =
    "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1?TrustedClientToken=6A5AA1D4EAFF4E9FB37E23D68491D6F4"; // ggignore
/// The service limits one SSML request; longer texts go in several requests.
pub const MAX_BYTES: usize = 4096;

pub fn chromium_major() -> &'static str {
    CHROMIUM_FULL_VERSION.split('.').next().unwrap_or("143")
}

pub fn user_agent() -> String {
    let m = chromium_major();
    format!(
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{m}.0.0.0 Safari/537.36 Edg/{m}.0.0.0"
    )
}

/// JavaScript-style date the service expects in X-Timestamp.
pub fn date_string(now: chrono::DateTime<chrono::Utc>) -> String {
    now.format("%a %b %d %Y %H:%M:%S GMT+0000 (Coordinated Universal Time)").to_string()
}

pub fn speech_config(timestamp: &str) -> String {
    format!(
        "X-Timestamp:{timestamp}\r\nContent-Type:application/json; charset=utf-8\r\nPath:speech.config\r\n\r\n\
         {{\"context\":{{\"synthesis\":{{\"audio\":{{\"metadataoptions\":{{\"sentenceBoundaryEnabled\":\"true\",\
         \"wordBoundaryEnabled\":\"false\"}},\"outputFormat\":\"audio-24khz-48kbitrate-mono-mp3\"}}}}}}}}\r\n"
    )
}

pub fn ssml(voice: &str, rate: &str, escaped_text: &str) -> String {
    format!(
        "<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='en-US'><voice name='{voice}'>\
         <prosody pitch='+0Hz' rate='{rate}' volume='+0%'>{escaped_text}</prosody></voice></speak>"
    )
}

pub fn ssml_message(request_id: &str, timestamp: &str, ssml: &str) -> String {
    // The trailing Z after the JavaScript date is how Edge sends it.
    format!("X-RequestId:{request_id}\r\nContent-Type:application/ssml+xml\r\nX-Timestamp:{timestamp}Z\r\nPath:ssml\r\n\r\n{ssml}")
}

/// Control characters the service refuses become spaces; then XML escaping.
pub fn prepare_text(text: &str) -> String {
    let cleaned: String = text
        .chars()
        .map(|c| match c as u32 {
            0..=8 | 11..=12 | 14..=31 => ' ',
            _ => c,
        })
        .collect();
    html_escape::encode_text(&cleaned).replace('\'', "&apos;").replace('"', "&quot;")
}

/// Splits escaped text into pieces of at most `limit` bytes, at a space or newline when there is
/// one, never inside a character or an XML entity.
pub fn split(text: &str, limit: usize) -> Vec<String> {
    let mut parts = Vec::new();
    let mut rest = text.trim();
    while rest.len() > limit {
        let mut cut = limit;
        while !rest.is_char_boundary(cut) {
            cut -= 1;
        }
        if let Some(space) = rest[..cut].rfind([' ', '\n']) {
            if space > 0 {
                cut = space;
            }
        }
        if let Some(amp) = rest[..cut].rfind('&') {
            if !rest[amp..cut].contains(';') {
                cut = amp.max(1);
            }
        }
        parts.push(rest[..cut].trim().to_string());
        rest = rest[cut..].trim_start();
    }
    if !rest.is_empty() {
        parts.push(rest.to_string());
    }
    parts.retain(|p| !p.is_empty());
    parts
}

/// A text frame's "Path:" header.
pub fn text_path(message: &str) -> Option<&str> {
    let head = message.split("\r\n\r\n").next()?;
    head.lines().find_map(|l| l.strip_prefix("Path:"))
}

/// Audio bytes of a binary frame (2-byte big-endian header length, headers, data); None for
/// frames without audio (the empty end-of-stream frame).
pub fn audio_payload(frame: &[u8]) -> Result<Option<&[u8]>, String> {
    if frame.len() < 2 {
        return Err("binary frame without a header length".into());
    }
    let header_len = usize::from(u16::from_be_bytes([frame[0], frame[1]]));
    if header_len + 2 > frame.len() {
        return Err("header length larger than the frame".into());
    }
    let headers = String::from_utf8_lossy(&frame[2..2 + header_len]);
    if !headers.contains("Path:audio") {
        return Err("binary frame that is not audio".into());
    }
    let data = &frame[2 + header_len..];
    let has_type = headers.contains("Content-Type:audio/mpeg");
    match (has_type, data.is_empty()) {
        (true, false) => Ok(Some(data)),
        (false, true) => Ok(None),
        (true, true) => Err("audio frame without data".into()),
        (false, false) => Err("audio data without a content type".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_on_spaces_and_keeps_entities() {
        let parts = split("aaaa bbbb &amp; cccc", 11);
        assert!(parts.iter().all(|p| p.len() <= 11));
        assert!(parts.iter().all(|p| !p.contains('&') || p.contains("&amp;")));
        assert_eq!(parts.join(" "), "aaaa bbbb &amp; cccc");
    }

    #[test]
    fn escapes_and_strips_control_chars() {
        assert_eq!(prepare_text("a\u{0b}<b>'"), "a &lt;b&gt;&apos;");
    }

    #[test]
    fn parses_audio_frames() {
        let head = b"X-RequestId:1\r\nContent-Type:audio/mpeg\r\nPath:audio\r\n";
        let mut frame = (head.len() as u16).to_be_bytes().to_vec();
        frame.extend_from_slice(head);
        frame.extend_from_slice(&[1, 2, 3]);
        assert_eq!(audio_payload(&frame).unwrap(), Some(&[1u8, 2, 3][..]));
        assert_eq!(text_path("X-RequestId:1\r\nPath:turn.end\r\n\r\n{}"), Some("turn.end"));
    }
}
