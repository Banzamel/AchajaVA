//! Async code (HTTP, websockets, IMAP) called from plain threads and command-line commands:
//! a small runtime per call, or the surrounding multi-threaded runtime when there is one.

use std::future::Future;

use crate::error::{AppError, Result};

/// Runs a future that returns our Result to completion.
pub fn block_on<T, F: Future<Output = Result<T>>>(fut: F) -> Result<T> {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread {
            return tokio::task::block_in_place(|| handle.block_on(fut));
        }
    }
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(AppError::Io)?;
    rt.block_on(fut)
}

/// The multi-threaded runtime of a long-running command (mail watcher, phone alerts).
pub fn multi_thread() -> Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(AppError::Io)
}

/// rustls needs one process-wide crypto provider; ring is the only one compiled in.
pub fn install_tls() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

/// An HTTP client with our user agent and a timeout.
pub fn http(timeout_secs: u64) -> Result<reqwest::Client> {
    install_tls();
    reqwest::Client::builder()
        .user_agent(format!("Achaja/{}", crate::VERSION))
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(AppError::from)
}
