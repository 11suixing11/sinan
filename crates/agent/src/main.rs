#![forbid(unsafe_code)]

#[cfg(target_os = "linux")]
use anyhow::Context;
use clap::{Parser, Subcommand};
#[cfg(target_os = "linux")]
use sinan_adapter_nodequality::NodeQualityAdapter;
#[cfg(target_os = "linux")]
use sinan_adapter_sdk::{Adapter, DiagnosticAdapter, Privileged, ServiceManager};
#[cfg(target_os = "linux")]
use sinan_adapter_singbox::SingboxAdapter;
#[cfg(target_os = "linux")]
use sinan_agent_core::{
    Config, identity,
    system::{SystemOps, SystemServiceManager},
    transport,
};
use std::path::PathBuf;
#[cfg(target_os = "linux")]
use std::{path::Path, sync::Arc};

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
    /// Verify a cached executable using only the compiled release trust roots.
    VerifyInstalled {
        #[arg(long)]
        binary: PathBuf,
        #[arg(long)]
        name: String,
        #[arg(long, value_parser = ["raw", "tar.gz"])]
        format: String,
    },
    /// Verify the next release using only the compiled release trust roots.
    VerifyRelease {
        #[arg(long)]
        proof_dir: PathBuf,
    },
    /// Check every installed or pending executable before replacing the Agent.
    VerifyCache,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let result = run_cli(cli).await;
    #[cfg(target_os = "linux")]
    if result.as_ref().is_err_and(|error| {
        error
            .downcast_ref::<sinan_agent_core::retirement::Retired>()
            .is_some()
    }) {
        std::process::exit(sinan_agent_core::retirement::RETIRED_EXIT_CODE);
    }
    result
}

#[cfg(not(target_os = "linux"))]
async fn run_cli(_cli: Cli) -> anyhow::Result<()> {
    anyhow::bail!("此平台仅提供编译产物和 CLI 检查；Agent 命令要求 Linux 和 systemd")
}

#[cfg(target_os = "linux")]
async fn run_cli(cli: Cli) -> anyhow::Result<()> {
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
            let adapters: Vec<Arc<dyn Adapter>> = vec![Arc::new(SingboxAdapter::new())];
            let diagnostics: Vec<Arc<dyn DiagnosticAdapter>> =
                vec![Arc::new(NodeQualityAdapter::new())];
            let services: Arc<dyn ServiceManager> =
                Arc::new(SystemServiceManager::new(privileged.clone()));
            transport::run_with_diagnostics(
                config,
                adapters,
                diagnostics,
                privileged,
                services,
                env!("CARGO_PKG_VERSION"),
            )
            .await
        }
        Command::Status => {
            let config = Config::load(&path)?;
            let status = transport::status(&config.status_socket).await?;
            println!("{}", serde_json::to_string_pretty(&status)?);
            Ok(())
        }
        Command::VerifyInstalled {
            binary,
            name,
            format,
        } => {
            sinan_agent_core::artifacts::verify_installed_binary(
                &absolute_path(&binary)?,
                &name,
                &format,
            )
            .await
        }
        Command::VerifyRelease { proof_dir } => {
            sinan_agent_core::artifacts::verify_release_directory(&absolute_path(&proof_dir)?).await
        }
        Command::VerifyCache => {
            let config = Config::load(&path)?;
            sinan_agent_core::artifacts::verify_cache(&config).await
        }
    }
}

#[cfg(target_os = "linux")]
fn absolute_path(path: &Path) -> anyhow::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}
