//! Speaking on a Google Cast / Google Home speaker: the speaker's default media receiver plays
//! the answer's MP3 from our small HTTP server; a stop request ends it early.

mod channel;
mod discover;
mod proto;
mod server;

pub use channel::insecure_config;
pub use discover::{discover, Speaker};

use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::error::Result;

const NS_CONNECTION: &str = "urn:x-cast:com.google.cast.tp.connection";
const NS_HEARTBEAT: &str = "urn:x-cast:com.google.cast.tp.heartbeat";
const NS_RECEIVER: &str = "urn:x-cast:com.google.cast.receiver";
const NS_MEDIA: &str = "urn:x-cast:com.google.cast.media";
const MEDIA_APP: &str = "CC1AD845";

fn payload(msg: &proto::CastMessage) -> Value {
    serde_json::from_str(&msg.payload).unwrap_or(Value::Null)
}

/// Waits for a message the predicate picks, answering heartbeats meanwhile.
fn wait_for(ch: &mut channel::Channel, deadline: Instant, pick: impl Fn(&proto::CastMessage, &Value) -> Option<Value>) -> Result<Option<Value>> {
    while Instant::now() < deadline {
        if let Some(msg) = ch.recv()? {
            let p = payload(&msg);
            if msg.namespace == NS_HEARTBEAT && p["type"] == "PING" {
                ch.send(&msg.source, NS_HEARTBEAT, r#"{"type":"PONG"}"#)?;
                continue;
            }
            if let Some(found) = pick(&msg, &p) {
                return Ok(Some(found));
            }
        }
    }
    Ok(None)
}

/// Plays MP3 bytes on the speaker at `host` until it ends or `stop()` says so.
pub fn play(mp3: &[u8], name: &str, host: &str, stop: &dyn Fn() -> bool) -> Result<()> {
    if host.trim().is_empty() {
        return Err(server::not_found(name, host));
    }
    let ip = server::local_ip(host)?;
    let srv = server::Server::start(ip, mp3.to_vec())?;
    let mut ch = channel::Channel::connect(host).map_err(|_| server::not_found(name, host))?;
    ch.send("receiver-0", NS_CONNECTION, r#"{"type":"CONNECT"}"#)?;
    ch.send("receiver-0", NS_RECEIVER, &json!({"type": "LAUNCH", "appId": MEDIA_APP, "requestId": 1}).to_string())?;
    let transport = wait_for(&mut ch, Instant::now() + Duration::from_secs(10), |_, p| {
        p["status"]["applications"].as_array()?.iter().find(|a| a["appId"] == MEDIA_APP).map(|a| a["transportId"].clone())
    })?
    .and_then(|t| t.as_str().map(str::to_string))
    .ok_or_else(|| server::not_found(name, host))?;
    ch.send(&transport, NS_CONNECTION, r#"{"type":"CONNECT"}"#)?;
    let load = json!({
        "type": "LOAD", "requestId": 2, "autoplay": true,
        "media": {"contentId": srv.url, "streamType": "BUFFERED", "contentType": "audio/mpeg"}
    });
    ch.send(&transport, NS_MEDIA, &load.to_string())?;
    // 48 kbit/s MP3: the length tells how long the speaker may take, plus time to start
    let seconds = mp3.len() as u64 * 8 / 48_000 + 20;
    let deadline = Instant::now() + Duration::from_secs(seconds);
    let (mut started, mut session, mut last_ping) = (false, Value::Null, Instant::now());
    while Instant::now() < deadline {
        if stop() {
            if !session.is_null() {
                let msg = json!({"type": "STOP", "mediaSessionId": session, "requestId": 3});
                let _ = ch.send(&transport, NS_MEDIA, &msg.to_string());
            }
            break;
        }
        if last_ping.elapsed() > Duration::from_secs(5) {
            ch.send("receiver-0", NS_HEARTBEAT, r#"{"type":"PING"}"#)?;
            last_ping = Instant::now();
        }
        let status = wait_for(&mut ch, Instant::now() + Duration::from_millis(200), |m, p| {
            (m.namespace == NS_MEDIA && p["type"] == "MEDIA_STATUS").then(|| p["status"][0].clone())
        })?;
        let Some(status) = status else { continue };
        if !status["mediaSessionId"].is_null() {
            session = status["mediaSessionId"].clone();
        }
        match status["playerState"].as_str() {
            Some("PLAYING") | Some("BUFFERING") => started = true,
            Some("IDLE") if started => break,
            _ => {}
        }
    }
    let _ = ch.send(&transport, NS_CONNECTION, r#"{"type":"CLOSE"}"#);
    drop(srv);
    Ok(())
}
