//! The microphone for the wake word: a stream that can be released and opened again. While any
//! program holds a Bluetooth headset's microphone, Windows keeps the headset in hands-free mode
//! (music and games sound thin), so pausing the listener closes the stream.
//! Audio arrives as 16 kHz mono i16 blocks of 0.25 s (what Vosk is fed).

use std::sync::mpsc::Sender;

use crate::error::Result;

pub const BLOCK: usize = 4000;

#[cfg(windows)]
pub struct Microphone {
    device: String,
    tx: Sender<Vec<i16>>,
    stream: Option<rodio::cpal::Stream>,
    pub name: String,
}

#[cfg(windows)]
impl Microphone {
    pub fn new(device: &str, tx: Sender<Vec<i16>>) -> Self {
        Self { device: device.to_string(), tx, stream: None, name: String::new() }
    }

    pub fn is_open(&self) -> bool {
        self.stream.is_some()
    }

    pub fn open(&mut self) -> Result<()> {
        use rodio::cpal::traits::{DeviceTrait, StreamTrait};
        use rodio::cpal::SampleFormat;

        use crate::error::AppError;

        if self.stream.is_some() {
            return Ok(());
        }
        let dev = super::devices::input_device(&self.device).ok_or_else(|| AppError::Audio("no microphone".into()))?;
        self.name = dev.name().unwrap_or_default();
        let supported = dev.default_input_config().map_err(|e| AppError::Audio(e.to_string()))?;
        let (channels, rate) = (supported.channels(), supported.sample_rate().0);
        let config = supported.config();
        let err = |e| eprintln!("microphone: {e}");
        let stream = match supported.sample_format() {
            SampleFormat::F32 => {
                let mut feed = Feeder::new(self.tx.clone(), channels, rate);
                dev.build_input_stream(&config, move |d: &[f32], _: &_| feed.push(d), err, None)
            }
            SampleFormat::I16 => {
                let mut feed = Feeder::new(self.tx.clone(), channels, rate);
                dev.build_input_stream(
                    &config,
                    move |d: &[i16], _: &_| feed.push(&d.iter().map(|s| f32::from(*s) / 32768.0).collect::<Vec<_>>()),
                    err,
                    None,
                )
            }
            other => return Err(AppError::Audio(format!("unsupported microphone format {other:?}"))),
        }
        .map_err(|e| AppError::Audio(e.to_string()))?;
        stream.play().map_err(|e| AppError::Audio(e.to_string()))?;
        self.stream = Some(stream);
        Ok(())
    }

    pub fn close(&mut self) {
        self.stream = None;
    }
}

/// Collects resampled audio into blocks for the recognizer thread.
#[cfg(windows)]
struct Feeder {
    tx: Sender<Vec<i16>>,
    channels: u16,
    rate: u32,
    resampler: super::clip::Resampler,
    buf: Vec<i16>,
}

#[cfg(windows)]
impl Feeder {
    fn new(tx: Sender<Vec<i16>>, channels: u16, rate: u32) -> Self {
        Self { tx, channels, rate, resampler: Default::default(), buf: Vec::with_capacity(BLOCK * 2) }
    }

    fn push(&mut self, data: &[f32]) {
        self.buf.extend(self.resampler.process(data, self.channels, self.rate));
        while self.buf.len() >= BLOCK {
            let block: Vec<i16> = self.buf.drain(..BLOCK).collect();
            let _ = self.tx.send(block);
        }
    }
}

/// Elsewhere there is no microphone (the core's tests run on Linux).
#[cfg(not(windows))]
pub struct Microphone {
    pub name: String,
}

#[cfg(not(windows))]
impl Microphone {
    pub fn new(_device: &str, _tx: Sender<Vec<i16>>) -> Self {
        Self { name: String::new() }
    }
    pub fn is_open(&self) -> bool {
        false
    }
    pub fn open(&mut self) -> Result<()> {
        Err(crate::error::AppError::Audio("the microphone needs Windows".into()))
    }
    pub fn close(&mut self) {}
}
