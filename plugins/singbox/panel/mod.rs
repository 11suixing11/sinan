pub mod accesses;
mod activity;
pub mod agent;
pub mod business;
pub mod chains;
pub mod deployments;
pub mod entitlements;
mod node_protocol;
mod node_settings;
pub mod nodes;
pub mod packages;
pub mod policies;
pub mod proxy_resources;
pub mod proxy_users;
pub mod publisher;
pub mod settings;
pub mod statistics;
pub mod subscription_parser;
pub mod subscription_sources;
pub mod subscriptions;
pub mod usage;

pub(super) use activity::runtime_activity_on;

use crate::AppState;
use axum::{
    Router,
    routing::{delete, get, post},
};

pub async fn run(state: AppState) {
    tokio::join!(
        publisher::run(state.clone()),
        subscription_sources::worker::run(state)
    );
}

pub fn router() -> Router<AppState> {
    let management = Router::new()
        .route("/statistics", get(statistics::summary))
        .route("/policy-groups", get(policies::list).post(policies::create))
        .route(
            "/policy-groups/{id}",
            axum::routing::put(policies::update).delete(policies::remove),
        )
        .route(
            "/package-groups",
            get(packages::list).post(packages::create),
        )
        .route(
            "/package-groups/{id}",
            axum::routing::put(packages::update).delete(packages::remove),
        )
        .route("/chains", get(chains::list).post(chains::create))
        .route("/chains/batch", post(chains::create_batch))
        .route("/chains/{id}", delete(chains::remove))
        .route("/proxy-resources", get(proxy_resources::list))
        .route(
            "/proxy-resources/{kind}/{id}",
            get(proxy_resources::get).delete(proxy_resources::remove),
        )
        .route(
            "/users/{id}/policy-groups",
            get(policies::user_get).put(policies::user_set),
        )
        .route("/users/{id}/package", post(packages::assign))
        .route("/users/{id}/entitlement", get(entitlements::get))
        .route("/servers", get(settings::list))
        .route("/servers/{id}", get(settings::get))
        .route("/servers/{id}/enable", post(settings::enable))
        .route("/servers/{id}/deployments", get(deployments::get))
        .route("/servers/{id}/deployments/check", post(deployments::check))
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
        .route("/users/{id}/subscription", get(subscriptions::preview))
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
        .nest(
            "/api/plugins/sing-box",
            management.merge(subscription_sources::routes()),
        )
        .route("/sub/{token}", get(subscriptions::get))
}
