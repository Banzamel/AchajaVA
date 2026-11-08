//! A blocking TLS connection to a Cast device (port 8009). The devices use self-signed
//! certificates, so the certificate is not checked: only the local network is ever addressed,
//! and nothing secret is sent (a link to an MP3 file).

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, ClientConnection, DigitallySignedStruct, SignatureScheme, StreamOwned};

use super::proto::{self, CastMessage};
use crate::error::{AppError, Result};

#[derive(Debug)]
pub struct AcceptAny;

impl ServerCertVerifier for AcceptAny {
    fn verify_server_cert(&self, _: &CertificateDer<'_>, _: &[CertificateDer<'_>], _: &ServerName<'_>, _: &[u8], _: UnixTime) -> std::result::Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(&self, _: &[u8], _: &CertificateDer<'_>, _: &DigitallySignedStruct) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }
    fn verify_tls13_signature(&self, _: &[u8], _: &CertificateDer<'_>, _: &DigitallySignedStruct) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        rustls::crypto::ring::default_provider().signature_verification_algorithms.supported_schemes()
    }
}

/// A TLS client configuration that accepts any certificate (Cast devices, mail servers with
/// verify_tls: false).
pub fn insecure_config() -> Arc<ClientConfig> {
    crate::runtime::install_tls();
    Arc::new(ClientConfig::builder().dangerous().with_custom_certificate_verifier(Arc::new(AcceptAny)).with_no_client_auth())
}

pub struct Channel {
    stream: StreamOwned<ClientConnection, TcpStream>,
    buf: Vec<u8>,
}

fn err(e: impl std::fmt::Display) -> AppError {
    AppError::Audio(format!("Cast: {e}"))
}

impl Channel {
    pub fn connect(host: &str) -> Result<Channel> {
        let addr = format!("{host}:8009");
        let sock = addr.parse().map_err(err)?;
        let tcp = TcpStream::connect_timeout(&sock, Duration::from_secs(5)).map_err(err)?;
        tcp.set_read_timeout(Some(Duration::from_millis(200))).map_err(err)?;
        let name = ServerName::try_from(host.to_string()).map_err(err)?;
        let conn = ClientConnection::new(insecure_config(), name).map_err(err)?;
        Ok(Channel { stream: StreamOwned::new(conn, tcp), buf: Vec::new() })
    }

    pub fn send(&mut self, destination: &str, namespace: &str, payload: &str) -> Result<()> {
        let msg = CastMessage {
            source: "sender-0".into(),
            destination: destination.into(),
            namespace: namespace.into(),
            payload: payload.into(),
        };
        self.stream.write_all(&proto::encode(&msg)).map_err(err)?;
        self.stream.flush().map_err(err)
    }

    /// The next message, or None when nothing arrived within the read timeout.
    pub fn recv(&mut self) -> Result<Option<CastMessage>> {
        loop {
            if self.buf.len() >= 4 {
                let len = u32::from_be_bytes([self.buf[0], self.buf[1], self.buf[2], self.buf[3]]) as usize;
                if self.buf.len() >= 4 + len {
                    let body: Vec<u8> = self.buf.drain(..4 + len).skip(4).collect();
                    return Ok(proto::decode(&body));
                }
            }
            let mut chunk = [0u8; 4096];
            match self.stream.read(&mut chunk) {
                Ok(0) => return Err(err("connection closed")),
                Ok(n) => self.buf.extend_from_slice(&chunk[..n]),
                Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {
                    return Ok(None)
                }
                Err(e) => return Err(err(e)),
            }
        }
    }
}
