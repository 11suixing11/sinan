use crate::AppState;
use std::time::Duration;

pub async fn run(state: AppState) {
    let mut poll = tokio::time::interval(Duration::from_secs(30));
    poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        poll.tick().await;
        if let Err(error) = crate::diagnostics::expire(&state).await {
            tracing::error!(%error, "diagnostic expiry cleanup failed");
        }
    }
}
