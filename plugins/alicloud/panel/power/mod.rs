mod api;
mod jobs;
mod policy;
mod scheduler;
mod state;
#[cfg(test)]
mod tests;

pub(super) use api::routes;
pub(super) use jobs::idle;
pub(super) use policy::Policy;
pub(super) use state::State;

use serde::Serialize;
use sqlx::{FromRow, types::Json};
use uuid::Uuid;

pub(super) async fn run(pool: sqlx::PgPool) {
    let mut timer = tokio::time::interval(std::time::Duration::from_secs(5));
    timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        timer.tick().await;
        if let Ok(cloud) = super::client::Cloud::new()
            && let Err(error) = jobs::tick(&pool, &cloud).await
        {
            tracing::warn!(%error,"Cloud power reconciliation failed");
        }
    }
}

#[derive(Serialize, FromRow)]
pub(super) struct Job {
    pub id: Uuid,
    pub resource_id: Uuid,
    pub account_revision: i64,
    pub resource_revision: i64,
    pub action: String,
    pub stop_mode: String,
    pub source: String,
    pub dedup_key: Option<String>,
    pub before_state: Json<State>,
    pub status: String,
    pub created_at: i64,
    pub expires_at: i64,
    pub updated_at: i64,
    pub next_check_at: i64,
    pub request_id: Option<String>,
    pub error_code: Option<String>,
}
