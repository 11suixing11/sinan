mod create;
mod models;
mod publication;
pub(crate) mod resources;
mod updates;

pub use create::batch;
pub use models::*;
pub use publication::{advance, compile_on, record_deployment_on};
pub use resources::{detail, list, remove, update};
pub use updates::{apply_versions, follow_updates};

pub fn router() -> axum::Router<crate::AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/proxy-resources", get(list))
        .route(
            "/proxy-resources/{kind}/{id}",
            get(detail).patch(update).delete(remove),
        )
        .route("/chains/batch", post(batch))
        .route(
            "/proxy-resources/chain/{id}/apply-node-versions",
            post(apply_versions),
        )
}
