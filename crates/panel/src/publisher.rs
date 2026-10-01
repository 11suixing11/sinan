//! Compatibility entry points for embedders using the previous public API.
pub use crate::plugins::singbox::publisher::publish_due;

pub async fn run(state: crate::AppState) {
    tokio::join!(
        crate::maintenance::run(state.clone()),
        crate::plugins::run(state)
    );
}
