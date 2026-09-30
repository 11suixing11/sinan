#![forbid(unsafe_code)]

use anyhow::Context;
use clap::{Parser, Subcommand};
#[cfg(target_os = "linux")]
use sinan_adapter_nodequality::NodeQualityAdapter;
use sinan_adapter_sdk::{Adapter, DiagnosticAdapter, Privileged, ServiceManager};
use sinan_adapter_singbox::SingboxAdapter;
use sinan_agent_core::{
    Config, identity,
    system::{ServiceBackend, SystemOps, SystemServiceManager},
    transport,
};
use std::path::PathBuf;
use std::{path::Path, sync::Arc};

#[derive(Parser)]
#[command(name = "sinan-agent", version, about = "Sinan 服务器代理")]
struct Cli {
    #[arg(long, global = true, default_value_os_t = sinan_agent_core::config::default_path())]
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
    Run {
        #[arg(long)]
        monitor_only: bool,
    },
    /// Query the running agent through its protected local socket.
    Status,
    /// Install native operating system services after enrollment.
    InstallService {
        #[arg(long)]
        monitor_only: bool,
    },
    /// Supervise the Agent and recover failed updates.
    Supervise {
        #[arg(long)]
        monitor_only: bool,
    },
    #[cfg(target_os = "linux")]
    #[command(hide = true)]
    RunJob {
        #[arg(long)]
        spec: PathBuf,
    },
    #[cfg(target_os = "linux")]
    #[command(hide = true)]
    ServiceJob {
        #[arg(long)]
        spec: PathBuf,
        #[arg(long)]
        status: bool,
    },
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
    if result.as_ref().is_err_and(|error| {
        error
            .downcast_ref::<sinan_agent_core::retirement::Retired>()
            .is_some()
    }) {
        std::process::exit(sinan_agent_core::retirement::RETIRED_EXIT_CODE);
    }
    result
}

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
        Command::Run { monitor_only } => {
            let config = Config::load(&path)?;
            let backend = if monitor_only {
                ServiceBackend::Unmanaged
            } else {
                ServiceBackend::detect()?
            };
            let adapters: Vec<Arc<dyn Adapter>> = if monitor_only {
                Vec::new()
            } else {
                vec![Arc::new(SingboxAdapter::new())]
            };
            #[cfg(target_os = "linux")]
            let diagnostics: Vec<Arc<dyn DiagnosticAdapter>> = if monitor_only {
                Vec::new()
            } else {
                vec![Arc::new(NodeQualityAdapter::new())]
            };
            #[cfg(not(target_os = "linux"))]
            let diagnostics: Vec<Arc<dyn DiagnosticAdapter>> = Vec::new();
            let services: Arc<dyn ServiceManager> = Arc::new(
                SystemServiceManager::new(privileged.clone(), backend).with_job_root(
                    config
                        .state_db
                        .parent()
                        .context("state has no parent")?
                        .join("service-jobs"),
                ),
            );
            tokio::select! {
                result = transport::run_with_diagnostics(config, adapters, diagnostics, privileged, services, env!("CARGO_PKG_VERSION")) => result,
                result = shutdown() => result,
            }
        }
        Command::Status => {
            let config = Config::load(&path)?;
            let mut status = transport::status(&config.status_socket).await?;
            status["update"] = serde_json::to_value(sinan_agent_core::upgrade::state(&config)?)?;
            println!("{}", serde_json::to_string_pretty(&status)?);
            Ok(())
        }
        Command::InstallService { monitor_only } => {
            let config = Config::load(&path)?;
            let descriptor = (!monitor_only).then(|| SingboxAdapter::new().describe());
            sinan_agent_core::system::deploy::install_services(&config, &path, descriptor.as_ref())
                .await?;
            println!("服务安装完成，可运行 sinan-agent status 查看状态。");
            Ok(())
        }
        Command::Supervise { monitor_only } => {
            let config = Config::load(&path)?;
            sinan_agent_core::upgrade::supervise(config, path, monitor_only, privileged).await
        }
        #[cfg(target_os = "linux")]
        Command::RunJob { spec } => sinan_agent_core::system::run_job(&absolute_path(&spec)?).await,
        #[cfg(target_os = "linux")]
        Command::ServiceJob { spec, status } => {
            let job: sinan_adapter_sdk::ServiceJob = serde_json::from_slice(&std::fs::read(spec)?)?;
            let services = SystemServiceManager::new(privileged, ServiceBackend::detect()?);
            if status {
                println!(
                    "{}",
                    serde_json::to_string(&services.job_status(&job.unit).await?)?
                );
                Ok(())
            } else {
                services.start_job(&job).await
            }
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

fn absolute_path(path: &Path) -> anyhow::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

async fn shutdown() -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        tokio::select! { _=term.recv()=>{}, result=tokio::signal::ctrl_c()=>{result?;} }
    }
    #[cfg(windows)]
    tokio::signal::ctrl_c().await?;
    Ok(())
}
