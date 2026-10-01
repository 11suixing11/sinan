pub mod accesses;
mod activity;
pub mod agent;
pub mod business;
pub mod deployments;
mod node_protocol;
pub mod nodes;
pub mod proxy_users;
pub mod publisher;
pub mod settings;
pub mod subscriptions;
pub mod usage;

pub(super) use activity::runtime_activity_on;

use crate::AppState;
use axum::{
    Router,
    routing::{delete, get, post},
};

pub fn router() -> Router<AppState> {
    let management = Router::new()
        .route("/servers", get(settings::list))
        .route("/servers/{id}", get(settings::get))
        .route("/servers/{id}/enable", post(settings::enable))
        .route("/servers/{id}/deployments", get(deployments::get))
        .route("/nodes", get(nodes::list).post(nodes::create))
        .route(
            "/nodes/{id}",
            get(nodes::get).patch(nodes::update).delete(nodes::remove),
        )
        .route("/users", get(proxy_users::list).post(proxy_users::create))
        .route(
            "/users/{id}",
            get(proxy_users::get)
                .patch(proxy_users::update)
                .delete(proxy_users::remove),
        )
        .route(
            "/users/{id}/subscription/reset",
            post(proxy_users::reset_subscription),
        )
        .route(
            "/users/{id}/accesses",
            get(accesses::list).post(accesses::grant),
        )
        .route(
            "/users/{user_id}/accesses/{node_id}",
            delete(accesses::revoke),
        )
        .route("/usage", get(usage::summary));
    Router::new()
        .nest("/api/plugins/sing-box", management)
        .route("/sub/{token}", get(subscriptions::get))
}
