//! Decoded audio in memory: the wake reply ("Słucham.") is kept trimmed of its silence, so it
//! sounds the instant the wake word is heard; and the conversion of microphone audio to the
//! 16 kHz mono 16-bit samples Vosk listens to.

#[derive(Debug, Clone)]
pub struct Clip {
    pub channels: u16,
    pub rate: u32,
    pub samples: Vec<f32>,
}

/// Samples between the first and the last sound above the threshold, with 30 ms before and
/// 100 ms after (like the Python version's numpy trim).
pub fn trim_silence(clip: &Clip, threshold: f32) -> Clip {
    let ch = usize::from(clip.channels.max(1));
    let loud: Vec<usize> = clip
        .samples
        .chunks(ch)
        .enumerate()
        .filter(|(_, frame)| frame.iter().any(|s| s.abs() > threshold))
        .map(|(i, _)| i)
        .collect();
    let (Some(first), Some(last)) = (loud.first(), loud.last()) else { return clip.clone() };
    let frames = clip.samples.len() / ch;
    let start = first.saturating_sub((0.03 * clip.rate as f32) as usize);
    let end = (last + (0.1 * clip.rate as f32) as usize).min(frames);
    Clip { channels: clip.channels, rate: clip.rate, samples: clip.samples[start * ch..end * ch].to_vec() }
}

/// Converts interleaved samples of any rate and channel count to 16 kHz mono i16.
/// `carry` keeps the fractional read position between calls, so blocks join seamlessly.
#[derive(Debug, Default)]
pub struct Resampler {
    pos: f64,
    last: f32,
}

impl Resampler {
    pub fn process(&mut self, input: &[f32], channels: u16, rate: u32) -> Vec<i16> {
        let ch = usize::from(channels.max(1));
        let mono: Vec<f32> = input.chunks(ch).map(|f| f.iter().sum::<f32>() / f.len() as f32).collect();
        if mono.is_empty() {
            return Vec::new();
        }
        let step = f64::from(rate) / 16_000.0;
        let mut out = Vec::with_capacity((mono.len() as f64 / step) as usize + 1);
        // pos is relative to this block; index -1 is the last sample of the previous block
        while self.pos < mono.len() as f64 - 1.0 {
            let i = self.pos.floor();
            let frac = (self.pos - i) as f32;
            let a = if i < 0.0 { self.last } else { mono[i as usize] };
            let b = mono[(i + 1.0) as usize];
            let v = a + (b - a) * frac;
            out.push((v.clamp(-1.0, 1.0) * 32767.0) as i16);
            self.pos += step;
        }
        self.pos -= mono.len() as f64;
        self.last = mono[mono.len() - 1];
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_leading_and_trailing_silence() {
        let mut samples = vec![0.0f32; 1000];
        samples.extend(vec![0.5f32; 100]);
        samples.extend(vec![0.0f32; 5000]);
        let clip = trim_silence(&Clip { channels: 1, rate: 1000, samples }, 0.01);
        assert_eq!(clip.samples.len(), 30 + 100 + 99);
    }

    #[test]
    fn resamples_48k_stereo_to_16k_mono() {
        let mut r = Resampler::default();
        let block = vec![0.25f32; 48_000 * 2 / 10]; // 100 ms of stereo
        let a = r.process(&block, 2, 48_000);
        let b = r.process(&block, 2, 48_000);
        assert!((a.len() + b.len()) as i64 - 3200 <= 1 && 3200 - (a.len() + b.len()) as i64 <= 2);
        assert!(a.iter().all(|s| (*s - 8191).abs() <= 1));
    }
}
