//! Playing speech on a local device: MP3 answers (stoppable) and the trimmed wake reply clip.

#[cfg(not(windows))]
use super::clip::Clip;
#[cfg(not(windows))]
use crate::error::{AppError, Result};

#[cfg(windows)]
mod imp {
    use std::io::Cursor;
    use std::time::Duration;

    use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};

    use super::super::clip::Clip;
    use super::super::devices::output_device;
    use crate::error::{AppError, Result};

    fn open(device: &str) -> Result<OutputStream> {
        let dev = output_device(device).ok_or_else(|| AppError::Audio("no audio output device".into()))?;
        let mut stream = OutputStreamBuilder::from_device(dev)
            .and_then(|b| b.open_stream_or_fallback())
            .map_err(|e| AppError::Audio(e.to_string()))?;
        stream.log_on_drop(false);
        Ok(stream)
    }

    pub fn play_mp3(mp3: &[u8], device: &str, stop: &dyn Fn() -> bool) -> Result<()> {
        let stream = open(device)?;
        let source = Decoder::new(Cursor::new(mp3.to_vec())).map_err(|e| AppError::Audio(e.to_string()))?;
        let sink = Sink::connect_new(stream.mixer());
        sink.append(source);
        while !sink.empty() {
            if stop() {
                sink.stop();
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Ok(())
    }

    pub fn decode(mp3: &[u8]) -> Result<Clip> {
        let source = Decoder::new(Cursor::new(mp3.to_vec())).map_err(|e| AppError::Audio(e.to_string()))?;
        let (channels, rate) = (source.channels(), source.sample_rate());
        Ok(Clip { channels, rate, samples: source.collect() })
    }

    pub fn play_clip(clip: &Clip, device: &str) -> Result<()> {
        let stream = open(device)?;
        let sink = Sink::connect_new(stream.mixer());
        sink.append(rodio::buffer::SamplesBuffer::new(clip.channels, clip.rate, clip.samples.clone()));
        sink.sleep_until_end();
        Ok(())
    }
}

#[cfg(windows)]
pub use imp::{decode, play_clip, play_mp3};

#[cfg(not(windows))]
pub fn play_mp3(_mp3: &[u8], _device: &str, _stop: &dyn Fn() -> bool) -> Result<()> {
    Err(AppError::Audio("audio playback needs Windows".into()))
}

#[cfg(not(windows))]
pub fn decode(_mp3: &[u8]) -> Result<Clip> {
    Err(AppError::Audio("audio decoding needs Windows".into()))
}

#[cfg(not(windows))]
pub fn play_clip(_clip: &Clip, _device: &str) -> Result<()> {
    Err(AppError::Audio("audio playback needs Windows".into()))
}

/// Windows' Beep (the signals of the listener); silent elsewhere.
pub fn beep(freq: u32, ms: u32) {
    #[cfg(windows)]
    // SAFETY: a plain synchronous Win32 call.
    unsafe {
        windows_sys::Win32::System::Diagnostics::Debug::Beep(freq, ms);
    }
    #[cfg(not(windows))]
    let _ = (freq, ms);
}
