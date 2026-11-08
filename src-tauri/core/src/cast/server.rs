//! A tiny HTTP server on the local network address facing the speaker: it serves the one MP3
//! the speaker is told to play, then stops with the playback.

use std::io::{Read, Write};
use std::net::{IpAddr, TcpListener, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::error::{AppError, Result};

/// Our address on the network that reaches `host` (no packet is sent).
pub fn local_ip(host: &str) -> Result<IpAddr> {
    let sock = UdpSocket::bind("0.0.0.0:0")?;
    sock.connect((host, 8009))?;
    Ok(sock.local_addr()?.ip())
}

pub struct Server {
    pub url: String,
    stop: Arc<AtomicBool>,
}

impl Server {
    pub fn start(ip: IpAddr, mp3: Vec<u8>) -> Result<Server> {
        let listener = TcpListener::bind((ip, 0))?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let data = Arc::new(mp3);
        std::thread::spawn(move || {
            while !flag.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut conn, _)) => {
                        let data = data.clone();
                        std::thread::spawn(move || {
                            let _ = conn.set_nonblocking(false);
                            let _ = conn.set_read_timeout(Some(Duration::from_secs(5)));
                            let mut head = [0u8; 2048];
                            let n = conn.read(&mut head).unwrap_or(0);
                            let is_head = head[..n].starts_with(b"HEAD");
                            let header = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: audio/mpeg\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                data.len()
                            );
                            let _ = conn.write_all(header.as_bytes());
                            if !is_head {
                                let _ = conn.write_all(&data);
                            }
                        });
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(50)),
                    Err(_) => break,
                }
            }
        });
        let url = match ip {
            IpAddr::V4(v4) => format!("http://{v4}:{port}/achaja.mp3"),
            IpAddr::V6(v6) => format!("http://[{v6}]:{port}/achaja.mp3"),
        };
        Ok(Server { url, stop })
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

pub fn not_found(name: &str, host: &str) -> AppError {
    AppError::Audio(format!("Nie znaleziono glosnika {name} ({host})"))
}
