//! Audio devices by the names Windows shows (WASAPI through cpal). config.json stores a part of
//! the name ("JBL"); an empty value means the system default.

#[cfg(windows)]
use rodio::cpal::traits::{DeviceTrait, HostTrait};

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct DeviceNames {
    pub input: Vec<String>,
    pub output: Vec<String>,
}

#[cfg(windows)]
pub fn list() -> DeviceNames {
    let host = rodio::cpal::default_host();
    let names = |devices: Option<Vec<rodio::cpal::Device>>| {
        let mut v: Vec<String> = devices.unwrap_or_default().iter().filter_map(|d| d.name().ok()).collect();
        v.sort();
        v.dedup();
        v
    };
    DeviceNames {
        input: names(host.input_devices().ok().map(Iterator::collect)),
        output: names(host.output_devices().ok().map(Iterator::collect)),
    }
}

#[cfg(not(windows))]
pub fn list() -> DeviceNames {
    DeviceNames::default()
}

/// The first device whose name contains `wanted` (case-insensitive); None = system default.
#[cfg(windows)]
fn pick(devices: impl Iterator<Item = rodio::cpal::Device>, wanted: &str) -> Option<rodio::cpal::Device> {
    let wanted = wanted.trim().to_lowercase();
    if wanted.is_empty() {
        return None;
    }
    devices.into_iter().find(|d| d.name().map(|n| n.to_lowercase().contains(&wanted)).unwrap_or(false))
}

#[cfg(windows)]
pub fn input_device(wanted: &str) -> Option<rodio::cpal::Device> {
    let host = rodio::cpal::default_host();
    pick(host.input_devices().ok()?, wanted).or_else(|| host.default_input_device())
}

#[cfg(windows)]
pub fn output_device(wanted: &str) -> Option<rodio::cpal::Device> {
    let host = rodio::cpal::default_host();
    pick(host.output_devices().ok()?, wanted).or_else(|| host.default_output_device())
}

/// True when a device with that part of the name exists (to report a missing microphone).
#[cfg(windows)]
pub fn input_exists(wanted: &str) -> bool {
    let host = rodio::cpal::default_host();
    host.input_devices().ok().and_then(|d| pick(d, wanted)).is_some()
}

#[cfg(not(windows))]
pub fn input_exists(_wanted: &str) -> bool {
    false
}
