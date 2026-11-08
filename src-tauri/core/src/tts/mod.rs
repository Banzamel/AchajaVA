//! Speech synthesis with Microsoft Edge's online neural voices (the service edge-tts uses),
//! implemented natively. Short repeated phrases ("Już sprawdzam.") come from a cache in
//! data\state\tts_cache, so they sound at once.

mod client;
mod drm;
mod drm_sha;
pub mod protocol;

use sha1::{Digest, Sha1};

use crate::error::Result;
use crate::paths::Paths;

pub use client::synthesize;

/// MP3 bytes for the text; phrases of up to 80 characters are cached.
pub fn synth_cached(paths: &Paths, text: &str, voice: &str, rate: &str) -> Result<Vec<u8>> {
    if text.chars().count() > 80 {
        return crate::runtime::block_on(synthesize(text, voice, rate));
    }
    let key = hex::encode(Sha1::digest(format!("{voice}|{rate}|{text}").as_bytes()));
    let cached = paths.state().join("tts_cache").join(format!("{}.mp3", &key[..16]));
    if let Ok(bytes) = std::fs::read(&cached) {
        if !bytes.is_empty() {
            return Ok(bytes);
        }
    }
    let bytes = crate::runtime::block_on(synthesize(text, voice, rate))?;
    if let Some(parent) = cached.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&cached, &bytes)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    // Talks to the real service: `cargo test -p achaja-core -- --ignored edge_service`.
    #[test]
    #[ignore]
    fn edge_service_returns_mp3() {
        let mp3 = crate::runtime::block_on(super::synthesize("Dzień dobry, tu Achaja.", "pl-PL-ZofiaNeural", "+15%")).unwrap();
        assert!(mp3.len() > 2000, "{} bytes", mp3.len());
        assert!(mp3[0] == 0xFF || &mp3[..3] == b"ID3");
    }
}
