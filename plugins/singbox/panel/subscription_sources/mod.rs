pub(crate) mod fetch;
mod jobs;
pub mod models;
mod mutations;
mod service;
mod snapshots;
pub mod worker;

pub use jobs::{cancel, get_job, refresh};
pub use mutations::{create, remove, update};
pub use service::{get, historical_nodes, history, list, nodes};

pub fn routes() -> axum::Router<crate::AppState> {
    use axum::routing::{get as route_get, post};
    axum::Router::new()
        .route("/subscription-sources", route_get(list).post(create))
        .route(
            "/subscription-sources/{id}",
            route_get(get).patch(update).delete(remove),
        )
        .route("/subscription-sources/{id}/refresh", post(refresh))
        .route("/subscription-sources/{id}/nodes", route_get(nodes))
        .route("/subscription-sources/{id}/revisions", route_get(history))
        .route(
            "/subscription-sources/{id}/revisions/{revision}/nodes",
            route_get(historical_nodes),
        )
        .route("/subscription-source-jobs/{id}", route_get(get_job))
        .route("/subscription-source-jobs/{id}/cancel", post(cancel))
        .layer(axum::extract::DefaultBodyLimit::max(3 * 1024 * 1024))
}
