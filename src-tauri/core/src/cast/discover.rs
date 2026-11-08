//! Finding Cast speakers on the network (mDNS "_googlecast._tcp"), for the settings window and
//! `achaja output discover`.

use std::time::{Duration, Instant};

use mdns_sd::{ServiceDaemon, ServiceEvent};

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct Speaker {
    pub name: String,
    pub host: String,
    pub model: String,
}

pub fn discover(seconds: u64) -> Vec<Speaker> {
    let Ok(daemon) = ServiceDaemon::new() else { return Vec::new() };
    let Ok(events) = daemon.browse("_googlecast._tcp.local.") else { return Vec::new() };
    let mut found: Vec<Speaker> = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(seconds);
    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        let Ok(event) = events.recv_timeout(left) else { break };
        if let ServiceEvent::ServiceResolved(info) = event {
            let Some(ip) = info.get_addresses_v4().into_iter().next() else { continue };
            let speaker = Speaker {
                name: info.get_property_val_str("fn").unwrap_or_else(|| info.get_hostname()).to_string(),
                host: ip.to_string(),
                model: info.get_property_val_str("md").unwrap_or("").to_string(),
            };
            if !found.iter().any(|s| s.host == speaker.host) {
                found.push(speaker);
            }
        }
    }
    let _ = daemon.shutdown();
    found
}
