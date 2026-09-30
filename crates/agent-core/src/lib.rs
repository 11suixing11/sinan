#![forbid(unsafe_code)]

#[cfg(test)]
#[path = "../../protocol/tests/support/release.rs"]
mod release_test_support;

pub mod artifacts;
pub mod config;
pub mod fake;
pub mod identity;
pub mod reconcile;
#[cfg(unix)]
pub mod retirement;
pub mod state;
#[cfg(unix)]
pub mod system;
pub mod telemetry;
#[cfg(unix)]
pub mod transport;
pub mod usage;

pub use config::Config;
pub use state::{SharedState, State};
