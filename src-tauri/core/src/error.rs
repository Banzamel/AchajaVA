//! The one error type of Achaja's native side. It crosses the IPC boundary as its message, so the
//! window shows a readable sentence, and the command line prints the same text after "BLAD:".

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Http(String),
    #[error("{0}")]
    Mail(String),
    #[error("{0}")]
    Audio(String),
    #[error("{0}")]
    Console(String),
    /// A readable message for the user (wrong argument, missing setting, refused action).
    #[error("{0}")]
    Msg(String),
}

impl AppError {
    pub fn msg(text: impl Into<String>) -> Self {
        AppError::Msg(text.into())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Http(e.to_string())
    }
}

impl From<zip::result::ZipError> for AppError {
    fn from(e: zip::result::ZipError) -> Self {
        AppError::Io(std::io::Error::other(e.to_string()))
    }
}

impl From<regex::Error> for AppError {
    fn from(e: regex::Error) -> Self {
        AppError::Msg(e.to_string())
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
