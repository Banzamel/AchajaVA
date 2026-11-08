//! Achaja's native logic, without any user interface: the voice (microphone, wake word,
//! speech, Cast), control of the hidden Claude Code session, the session hooks, project agents,
//! mail, Home Assistant, the plan and background jobs, backups and the command line
//! (`Achaja.exe <command>`). The Tauri app (src-tauri/src) only wires it to the window.

pub mod agents;
pub mod audio_routing;
pub mod backup;
pub mod cast;
pub mod chat;
pub mod claude;
pub mod cli;
pub mod config;
pub mod console;
pub mod error;
pub mod gmail;
pub mod homeassistant;
pub mod hooks;
pub mod json;
pub mod listener;
pub mod logfile;
pub mod mail;
pub mod paths;
pub mod phone;
pub mod plan;
pub mod process;
pub mod runtime;
pub mod settings;
pub mod state;
pub mod stt;
pub mod text;
pub mod tts;
pub mod update;
pub mod voice;

/// The version of this build (package.json).
pub const VERSION: &str = env!("ACHAJA_VERSION");
