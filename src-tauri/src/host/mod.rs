//! The assistant hosted by the window process: its state, the listener thread, the console
//! reader, the supervisor and the shutdown of everything.

pub mod listener;
pub mod screen;
pub mod shutdown;
pub mod state;
pub mod supervisor;

pub use state::AppState;
