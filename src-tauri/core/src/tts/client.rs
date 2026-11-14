//! One synthesis over the Edge read-aloud websocket: connect with the DRM token, send the speech
//! config and the SSML, collect the MP3 frames until "turn.end". A rejected handshake (403) with
//! the server's Date corrects the clock skew once and tries again, like edge-tts; a dropped
//! connection is tried again with a short back-off.

use std::sync::atomic::{AtomicI64, Ordering};

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

use super::{drm, protocol};
use crate::error::{AppError, Result};
use crate::retry::{self, Fail};

static CLOCK_SKEW: AtomicI64 = AtomicI64::new(0);

fn now_corrected() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now() + chrono::Duration::seconds(CLOCK_SKEW.load(Ordering::Relaxed))
}

fn connect_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

fn request(url: &str) -> Result<tokio_tungstenite::tungstenite::handshake::client::Request> {
    let mut req = url.into_client_request().map_err(|e| AppError::Http(e.to_string()))?;
    let muid = hex::encode_upper(rand::random::<[u8; 16]>());
    let headers = [
        ("Pragma", "no-cache".to_string()),
        ("Cache-Control", "no-cache".to_string()),
        ("Origin", "chrome-extension://jdiccldimpdaibmpdkjnbmckianbfold".to_string()),
        ("User-Agent", protocol::user_agent()),
        ("Accept-Encoding", "gzip, deflate, br, zstd".to_string()),
        ("Accept-Language", "en-US,en;q=0.9".to_string()),
        ("Cookie", format!("muid={muid};")),
    ];
    for (name, value) in headers {
        let value = HeaderValue::from_str(&value).map_err(|e| AppError::Http(e.to_string()))?;
        req.headers_mut().insert(name, value);
    }
    Ok(req)
}

fn url() -> String {
    let token = drm::sec_ms_gec(now_corrected().timestamp() as f64);
    format!(
        "{}&ConnectionId={}&Sec-MS-GEC={token}&Sec-MS-GEC-Version=2-{}",
        protocol::WSS_URL,
        connect_id(),
        protocol::CHROMIUM_FULL_VERSION
    )
}

/// Remembers how far our clock is from the server's (from the Date of a refused handshake).
fn adjust_skew(err: &WsError) -> bool {
    let WsError::Http(resp) = err else { return false };
    let Some(date) = resp.headers().get("Date").and_then(|d| d.to_str().ok()) else { return false };
    let Ok(server) = chrono::DateTime::parse_from_rfc2822(date) else { return false };
    let skew = server.timestamp() - chrono::Utc::now().timestamp();
    CLOCK_SKEW.store(skew, Ordering::Relaxed);
    true
}

async fn synth_part(escaped: &str, voice: &str, rate: &str) -> Result<Vec<u8>> {
    let mut attempt = 0;
    let (mut ws, _) = loop {
        match tokio_tungstenite::connect_async(request(&url())?).await {
            Ok(ok) => break ok,
            Err(e) if attempt == 0 && adjust_skew(&e) => attempt += 1,
            Err(e) => return Err(AppError::Http(format!("edge-tts: {e}"))),
        }
    };
    let stamp = protocol::date_string(now_corrected());
    let send = |m: String| Message::Text(m.into());
    ws.send(send(protocol::speech_config(&stamp))).await.map_err(|e| AppError::Http(e.to_string()))?;
    let body = protocol::ssml(voice, rate, escaped);
    ws.send(send(protocol::ssml_message(&connect_id(), &stamp, &body)))
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    let mut audio = Vec::new();
    while let Some(msg) = ws.next().await {
        match msg.map_err(|e| AppError::Http(e.to_string()))? {
            Message::Text(text) => {
                if protocol::text_path(&text) == Some("turn.end") {
                    break;
                }
            }
            Message::Binary(frame) => {
                if let Some(data) = protocol::audio_payload(&frame).map_err(AppError::Http)? {
                    audio.extend_from_slice(data);
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
    let _ = ws.close(None).await;
    if audio.is_empty() {
        return Err(AppError::Http("edge-tts: no audio received (check the voice name)".into()));
    }
    Ok(audio)
}

/// MP3 (24 kHz mono) of the whole text; rate like "+15%".
pub async fn synthesize(text: &str, voice: &str, rate: &str) -> Result<Vec<u8>> {
    crate::runtime::install_tls();
    let mut out = Vec::new();
    for part in protocol::split(&protocol::prepare_text(text), protocol::MAX_BYTES) {
        // a dropped connection is tried again; no audio means a wrong voice name, which stays wrong
        let part = part.as_str();
        let audio = retry::run(&retry::NETWORK, move || async move {
            synth_part(part, voice, rate).await.map_err(|e| if e.to_string().contains("no audio") { Fail::Fatal(e) } else { Fail::Transient(e) })
        })
        .await?;
        out.extend(audio);
    }
    Ok(out)
}
