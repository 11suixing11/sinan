#![forbid(unsafe_code)]

pub mod artifacts;
pub mod config;
pub mod fake;
pub mod identity;
pub mod reconcile;
pub mod state;
#[cfg(unix)]
pub mod system;
#[cfg(windows)]
#[path = "system/windows.rs"]
pub mod system;
pub mod tasks;
pub mod telemetry;
pub mod transport;
pub mod upgrade;
pub mod usage;

pub use config::Config;
pub use state::{SharedState, State};
