#![forbid(unsafe_code)]

mod cli;
mod engine;
mod journal;
mod model;

pub use cli::{Command, Options, parse};
pub use engine::run;
pub use journal::Journal;
pub use model::{IpVersion, Report, Snapshot, Target};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SOURCE_COMMIT: Option<&str> = option_env!("SINAN_NATIVE_TCP_SOURCE_COMMIT");
pub const INPUT_LIMIT: usize = 16 * 1024;
pub const OUTPUT_LIMIT: usize = 64 * 1024;

#[cfg(test)]
mod tests;
