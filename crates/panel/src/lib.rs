#![forbid(unsafe_code)]

pub mod accesses;
pub mod agent_api;
pub mod artifacts;
pub mod auth;
pub mod business;
pub mod config;
pub mod deployments;
pub mod diagnostics;
pub mod error;
pub mod frontend;
pub mod ip_quality;
pub mod nodes;
pub mod publisher;
pub mod releases;
pub mod servers;
pub mod subscriptions;
pub mod usage;
pub mod users;

use axum::{
    Router,
    routing::{get, post},
};
use config::Config;
use sinan_protocol::Envelope;
use sqlx::PgPool;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{RwLock, Semaphore, mpsc};
use uuid::Uuid;

#[derive(Clone)]
pub struct AgentConnection {
    pub id: Uuid,
    pub sender: mpsc::Sender<Envelope>,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub login_permits: Arc<Semaphore>,
    pub quality_permits: Arc<Semaphore>,
    pub release_permits: Arc<Semaphore>,
    pub release_keys: Option<Arc<sinan_protocol::release::TrustedKeys>>,
    pub config: Arc<Config>,
    pub connections: Arc<RwLock<HashMap<i64, AgentConnection>>>,
}

impl AppState {
    pub async fn new(pool: PgPool, config: Config) -> anyhow::Result<Self> {
        sqlx::migrate!().run(&pool).await?;
        auth::ensure_admin(&pool, config.admin_password.as_deref()).await?;
        Ok(Self {
            pool,
            login_permits: Arc::new(Semaphore::new(4)),
            quality_permits: Arc::new(Semaphore::new(2)),
            release_permits: Arc::new(Semaphore::new(1)),
            release_keys: sinan_protocol::release::TrustedKeys::compiled()
                .ok()
                .map(Arc::new),
            config: Arc::new(config),
            connections: Arc::default(),
        })
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/api/login", post(auth::login))
        .route("/api/logout", post(auth::logout))
        .route("/api/me", get(auth::me))
        .route("/api/servers", get(servers::list).post(servers::create))
        .route(
            "/api/servers/{id}",
            get(servers::get)
                .patch(servers::update)
                .delete(servers::remove),
        )
        .route(
            "/api/servers/{id}/enrollment",
            post(servers::issue_enrollment),
        )
        .route("/api/servers/{id}/deployments", get(deployments::get))
        .route("/api/servers/{id}/node-quality", get(diagnostics::get))
        .route(
            "/api/servers/{id}/node-quality/refresh",
            post(ip_quality::refresh),
        )
        .route(
            "/api/servers/{id}/node-quality/reports",
            post(diagnostics::create),
        )
        .route("/api/nodes", get(nodes::list).post(nodes::create))
        .route(
            "/api/nodes/{id}",
            get(nodes::get).patch(nodes::update).delete(nodes::remove),
        )
        .route("/api/users", get(users::list).post(users::create))
        .route(
            "/api/users/{id}",
            get(users::get).patch(users::update).delete(users::remove),
        )
        .route(
            "/api/users/{id}/accesses",
            get(accesses::list).post(accesses::grant),
        )
        .route(
            "/api/users/{user_id}/accesses/{node_id}",
            axum::routing::delete(accesses::revoke),
        )
        .route("/api/usage", get(usage::summary))
        .route("/sub/{token}", get(subscriptions::get))
        .route("/api/agent/v1/enroll", post(servers::enroll))
        .route("/api/agent/v1/ws", get(agent_api::websocket))
        .route("/api/agent/v1/manifest", get(agent_api::manifest))
        .route("/api/agent/v1/diagnostics", get(diagnostics::pending))
        .route("/api/agent/v1/diagnostics/{id}", post(diagnostics::update))
        .route("/api/agent/v1/bundles/{rev}", get(agent_api::bundle))
        .route(
            "/api/agent/v1/artifacts/{name}/{version}/{arch}",
            get(artifacts::download),
        )
        .route("/api/artifacts", get(artifacts::list))
        .route("/api/artifacts/import-release", post(releases::import))
        .route("/api/bootstrap/{version}/{arch}", get(artifacts::bootstrap))
        .route("/install.sh", get(artifacts::install_script))
        .fallback(frontend::serve)
        .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024))
        .with_state(state)
}
