#![forbid(unsafe_code)]

use sinan_panel::{AppState, config::Config, router};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let config = Config::from_env()?;
    let listen = config.listen;
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;
    let state = AppState::new(pool, config).await?;
    let listener = tokio::net::TcpListener::bind(listen).await?;
    tracing::info!(address = %listener.local_addr()?, "panel started");
    let publisher = tokio::spawn(sinan_panel::publisher::run(state.clone()));
    let result = axum::serve(
        listener,
        router(state).into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await;
    publisher.abort();
    result?;
    Ok(())
}
