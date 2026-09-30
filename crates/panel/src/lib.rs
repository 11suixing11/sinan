#![forbid(unsafe_code)]

pub mod agent_api;
pub mod agent_updates;
pub mod artifacts;
pub mod auth;
pub mod commands;
pub mod config;
pub mod diagnostic_plugins;
pub mod diagnostics;
pub mod error;
pub mod frontend;
pub mod ip_quality;
pub mod maintenance;
pub mod plugins;
pub mod probes;
// Compatibility exports preserve the public Rust embedding API.
pub use plugins::singbox::proxy_users as users;
pub use plugins::singbox::{accesses, business, deployments, nodes, subscriptions, usage};
pub mod publisher;
pub mod releases;
pub mod retirement;
pub mod servers;
pub mod telemetry;

use axum::{
    Router,
    routing::{get, post},
};
use config::Config;
use sinan_protocol::Envelope;
use sqlx::PgPool;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, RwLock, Semaphore, mpsc};
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
    pub quality_providers: Arc<ip_quality::ProviderRegistry>,
    pub release_permits: Arc<Semaphore>,
    pub release_keys: Option<Arc<sinan_protocol::release::TrustedKeys>>,
    pub config: Arc<Config>,
    pub connections: Arc<RwLock<HashMap<i64, AgentConnection>>>,
    pub device_lifecycle: Arc<Mutex<()>>,
}

impl AppState {
    pub async fn new(pool: PgPool, config: Config) -> anyhow::Result<Self> {
        sqlx::migrate!().run(&pool).await?;
        auth::ensure_admin(&pool, config.admin_password.as_deref()).await?;
        Ok(Self {
            pool,
            login_permits: Arc::new(Semaphore::new(4)),
            quality_permits: Arc::new(Semaphore::new(2)),
            quality_providers: Arc::new(ip_quality::ProviderRegistry::from_env()),
            release_permits: Arc::new(Semaphore::new(1)),
            release_keys: sinan_protocol::release::TrustedKeys::compiled()
                .ok()
                .map(Arc::new),
            config: Arc::new(config),
            connections: Arc::default(),
            device_lifecycle: Arc::default(),
        })
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/api/login", post(auth::login))
        .route("/api/logout", post(auth::logout))
        .route("/api/me", get(auth::me))
        .route("/api/security/totp", get(auth::totp_status))
        .route("/api/security/totp/setup", post(auth::totp_setup))
        .route("/api/security/totp/confirm", post(auth::totp_confirm))
        .route("/api/security/totp/disable", post(auth::totp_disable))
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
        .route(
            "/api/servers/{id}/agent-settings",
            get(telemetry::settings).patch(telemetry::update_settings),
        )
        .route("/api/servers/{id}/metrics", get(telemetry::history))
        .route(
            "/api/servers/{id}/commands",
            get(commands::list).post(commands::create),
        )
        .route(
            "/api/servers/{id}/probes",
            get(probes::list).post(probes::create),
        )
        .route(
            "/api/servers/{id}/probes/{probe}",
            axum::routing::patch(probes::update).delete(probes::remove),
        )
        .route("/api/servers/{id}/probe-results", get(probes::history))
        .route(
            "/api/servers/{id}/diagnostics",
            get(diagnostics::service::get),
        )
        .route(
            "/api/servers/{id}/diagnostics/{plugin}",
            post(diagnostics::service::create),
        )
        .route("/api/servers/{id}/ip-quality", get(ip_quality::get))
        .route(
            "/api/servers/{id}/ip-quality/refresh",
            post(ip_quality::refresh),
        )
        .route(
            "/api/servers/{id}/node-quality",
            get(diagnostics::legacy_get),
        )
        .route(
            "/api/servers/{id}/node-quality/refresh",
            post(ip_quality::refresh),
        )
        .route(
            "/api/servers/{id}/node-quality/reports",
            get(diagnostics::get).post(diagnostics::create),
        )
        .route(
            "/api/servers/{id}/diagnostics/{job}/cancel",
            post(diagnostics::cancellation::request),
        )
        .merge(plugins::router())
        .route("/api/agent/v1/enroll", post(servers::enroll))
        .route("/api/agent/v1/ws", get(agent_api::websocket))
        .route(
            "/api/agent/v1/retirement/receipt",
            post(retirement::receipt),
        )
        .route("/api/agent/v1/manifest", get(agent_api::manifest))
        .route("/api/agent/v1/settings", get(telemetry::agent_settings))
        .route("/api/agent/v1/update", get(agent_updates::available))
        .route("/api/agent/v1/telemetry", post(telemetry::ingest))
        .route("/api/agent/v1/commands", get(commands::pending))
        .route("/api/agent/v1/commands/{id}", post(commands::complete))
        .route("/api/agent/v1/probes", get(probes::agent_list))
        .route("/api/agent/v1/probe-results", post(probes::ingest))
        .route("/api/agent/v1/diagnostics", get(diagnostics::pending))
        .route(
            "/api/agent/v1/diagnostics/cancellations",
            get(diagnostics::cancellation::pending),
        )
        .route(
            "/api/agent/v1/diagnostics/{id}/cancel-confirmation",
            post(diagnostics::cancellation::confirm),
        )
        .route("/api/agent/v1/diagnostics/{id}", post(diagnostics::update))
        .route(
            "/api/agent/v1/diagnostics/{id}/sections",
            post(diagnostics::upload_section),
        )
        .route("/api/agent/v1/bundles/{rev}", get(agent_api::bundle))
        .route(
            "/api/agent/v1/artifacts/{name}/{version}/{arch}",
            get(artifacts::download),
        )
        .route("/api/artifacts", get(artifacts::list))
        .route("/api/artifacts/import-release", post(releases::import))
        .route("/api/bootstrap/{version}/{arch}", get(artifacts::bootstrap))
        .route("/install.sh", get(artifacts::install_script))
        .route("/install.ps1", get(artifacts::install_powershell))
        .fallback(frontend::serve)
        .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024))
        .with_state(state)
}
