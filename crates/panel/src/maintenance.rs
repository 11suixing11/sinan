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
        if let Err(error) =
            crate::server_assets::renew_due(&state.pool, sinan_protocol::now_timestamp()).await
        {
            tracing::error!(%error, "server expiry renewal failed");
        }
        let now = sinan_protocol::now_timestamp();
        if let Err(error) = crate::notifications::evaluate(&state.pool, state.started_at, now).await
        {
            tracing::error!(%error, "offline event evaluation failed");
        }
        if let Err(error) = crate::notifications::dispatch(&state.pool, now).await {
            tracing::error!(%error, "notification delivery failed");
        }
    }
}
