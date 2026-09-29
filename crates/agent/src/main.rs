#![forbid(unsafe_code)]

use anyhow::{ensure, Context};
use clap::{Parser, Subcommand};
use sinan_adapter_sdk::{Adapter, Privileged, ServiceManager};
use sinan_agent_core::{
    identity,
    system::{SystemOps, SystemServiceManager},
    transport, Config,
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Parser)]
#[command(name = "sinan-agent", version, about = "Sinan 服务器代理")]
struct Cli {
    #[arg(long, global = true, default_value = "/etc/sinan/agent.toml")]
    config: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Enroll this device with its control panel.
    Enroll {
        #[arg(long)]
        panel: String,
        #[arg(long)]
        token: String,
    },
    /// Maintain connectivity, telemetry, accounting, and desired state.
    Run,
    /// Query the running agent through its protected local socket.
    Status,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    ensure!(
        cfg!(target_os = "linux"),
        "the production Agent requires Linux and systemd"
    );
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with_writer(std::io::stderr)
        .init();
    let path = absolute_path(&cli.config)?;
    let privileged: Arc<dyn Privileged> = Arc::new(SystemOps);
    match cli.command {
        Command::Enroll { panel, token } => {
            let mut config = if path.try_exists()? {
                Config::load(&path)?
            } else {
                Config::default()
            };
            config.panel_url = panel.trim_end_matches('/').to_owned();
            config.validate()?;
            let serialized =
                toml::to_string_pretty(&config).context("serialize Agent configuration")?;
            let server_id = identity::enroll(&config, &token).await?;
            privileged
                .write_file(&path, serialized.as_bytes(), 0o600, None)
                .await
                .context("save enrolled Agent configuration")?;
            println!("注册成功，服务器 ID：{server_id}");
            Ok(())
        }
        Command::Run => {
            let config = Config::load(&path)?;
            let adapters: Vec<Arc<dyn Adapter>> = Vec::new();
            let services: Arc<dyn ServiceManager> =
                Arc::new(SystemServiceManager::new(privileged.clone()));
            transport::run(config, adapters, privileged, services).await
        }
        Command::Status => {
            let config = Config::load(&path)?;
            let status = transport::status(&config.status_socket).await?;
            println!("{}", serde_json::to_string_pretty(&status)?);
            Ok(())
        }
    }
}

fn absolute_path(path: &Path) -> anyhow::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}
