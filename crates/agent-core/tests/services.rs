#![forbid(unsafe_code)]
#![cfg(unix)]

use anyhow::{Result, bail};
use sinan_adapter_sdk::{
    BoxFuture, CommandOutput, JobStatus, Privileged, ServiceJob, ServiceManager,
};
use sinan_agent_core::system::{ServiceBackend, SystemServiceManager};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Default)]
struct RecordingOps {
    calls: Mutex<Vec<(PathBuf, Vec<String>)>>,
    output: Mutex<CommandOutput>,
    unavailable: Mutex<bool>,
}

impl RecordingOps {
    fn successful() -> Arc<Self> {
        Arc::new(Self {
            output: Mutex::new(CommandOutput {
                success: true,
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

impl Privileged for RecordingOps {
    fn execute<'a>(
        &'a self,
        program: &'a Path,
        args: &'a [String],
    ) -> BoxFuture<'a, CommandOutput> {
        Box::pin(async move {
            self.calls
                .lock()
                .unwrap()
                .push((program.into(), args.into()));
            if *self.unavailable.lock().unwrap() {
                bail!("service command unavailable");
            }
            Ok(self.output.lock().unwrap().clone())
        })
    }
    fn create_dir<'a>(
        &'a self,
        _path: &'a Path,
        _mode: u32,
        _group: Option<&'a str>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async { bail!("unexpected filesystem operation") })
    }
    fn write_file<'a>(
        &'a self,
        _path: &'a Path,
        _bytes: &'a [u8],
        _mode: u32,
        _group: Option<&'a str>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async { bail!("unexpected filesystem operation") })
    }
    fn atomic_symlink<'a>(&'a self, _link: &'a Path, _target: &'a Path) -> BoxFuture<'a, ()> {
        Box::pin(async { bail!("unexpected filesystem operation") })
    }
    fn remove_symlink<'a>(&'a self, _link: &'a Path) -> BoxFuture<'a, ()> {
        Box::pin(async { bail!("unexpected filesystem operation") })
    }
    fn install_archive<'a>(
        &'a self,
        _archive: &'a Path,
        _directory: &'a Path,
        _binary_name: &'a str,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async { bail!("unexpected filesystem operation") })
    }
}

#[tokio::test]
async fn routes_runtime_lifecycle_to_selected_init_without_changing_instance() -> Result<()> {
    let unit = "example-runtime@main.service";
    for backend in [ServiceBackend::Systemd, ServiceBackend::OpenRc] {
        let ops = RecordingOps::successful();
        let services = SystemServiceManager::new(ops.clone(), backend);
        services.restart(unit).await?;
        services.reload(unit).await?;
        assert!(services.is_active(unit).await?);
        services.stop(unit).await?;
        let calls = ops.calls.lock().unwrap();
        let expected = match backend {
            ServiceBackend::Systemd => vec![
                vec!["restart", "--", unit],
                vec!["reload", "--", unit],
                vec!["is-active", "--quiet", "--", unit],
                vec!["stop", "--", unit],
            ],
            ServiceBackend::OpenRc => vec![
                vec!["--", "example-runtime@main", "restart"],
                vec!["--", "example-runtime@main", "reload"],
                vec!["--", "example-runtime@main", "status"],
                vec!["--", "example-runtime@main", "stop"],
            ],
        };
        let program = match backend {
            ServiceBackend::Systemd => "systemctl",
            ServiceBackend::OpenRc => "rc-service",
        };
        for ((actual_program, args), expected_args) in calls.iter().zip(expected) {
            assert_eq!(actual_program, Path::new(program));
            assert_eq!(args, &expected_args);
        }
        assert_eq!(calls.len(), 4);
    }
    Ok(())
}

#[tokio::test]
async fn stopped_services_are_inactive_but_execution_errors_are_propagated() -> Result<()> {
    for backend in [ServiceBackend::Systemd, ServiceBackend::OpenRc] {
        let ops = Arc::new(RecordingOps::default());
        let services = SystemServiceManager::new(ops.clone(), backend);
        assert!(!services.is_active("example-runtime.service").await?);
        assert!(services.restart("example-runtime.service").await.is_err());
        assert!(services.reload("example-runtime.service").await.is_err());
        assert!(services.stop("example-runtime.service").await.is_err());
        *ops.unavailable.lock().unwrap() = true;
        assert!(services.is_active("example-runtime.service").await.is_err());
    }
    Ok(())
}

#[tokio::test]
async fn rejects_untrusted_service_names_before_any_privileged_command() {
    for backend in [ServiceBackend::Systemd, ServiceBackend::OpenRc] {
        let ops = RecordingOps::successful();
        let services = SystemServiceManager::new(ops.clone(), backend);
        let oversized = "x".repeat(256);
        for unit in [
            "",
            ".",
            "..",
            ".service",
            "-option",
            "../other",
            "two services",
            "name;cmd",
            &oversized,
        ] {
            assert!(services.restart(unit).await.is_err());
            assert!(services.reload(unit).await.is_err());
            assert!(services.stop(unit).await.is_err());
            assert!(services.is_active(unit).await.is_err());
        }
        assert!(ops.calls.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn openrc_rejects_diagnostic_jobs_without_invoking_systemd() {
    let ops = RecordingOps::successful();
    let services = SystemServiceManager::new(ops.clone(), ServiceBackend::OpenRc);
    let job = ServiceJob {
        unit: format!("sinan-diagnostic-{}.service", uuid::Uuid::new_v4()),
        program: "/bin/true".into(),
        args: Vec::new(),
        working_directory: "/tmp".into(),
        timeout_secs: 10,
    };
    let error = services.start_job(&job).await.unwrap_err();
    assert!(error.to_string().contains("OpenRC"));
    let error = services.job_status(&job.unit).await.unwrap_err();
    assert!(error.to_string().contains("OpenRC"));
    assert!(ops.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn systemd_diagnostic_jobs_keep_independent_supervision_and_status() -> Result<()> {
    let ops = RecordingOps::successful();
    let services = SystemServiceManager::new(ops.clone(), ServiceBackend::Systemd);
    let job = ServiceJob {
        unit: format!("sinan-diagnostic-{}.service", uuid::Uuid::new_v4()),
        program: "/bin/true".into(),
        args: Vec::new(),
        working_directory: "/tmp".into(),
        timeout_secs: 10,
    };
    services.start_job(&job).await?;
    ops.output.lock().unwrap().stdout = "LoadState=loaded\nActiveState=active\nSubState=exited\nResult=success\nExecMainStatus=0\nExecMainCode=1\nExecMainStartTimestampMonotonic=1\n".into();
    assert_eq!(services.job_status(&job.unit).await?, JobStatus::Succeeded);
    let calls = ops.calls.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].0, Path::new("systemd-run"));
    assert!(
        calls[0]
            .1
            .contains(&"--property=KillMode=control-group".into())
    );
    assert!(calls[0].1.contains(&"--property=PrivateMounts=yes".into()));
    assert!(
        calls[0]
            .1
            .contains(&"--property=TimeoutStartSec=10s".into())
    );
    assert_eq!(calls[1].0, Path::new("systemctl"));
    assert_eq!(calls[1].1.last(), Some(&job.unit));
    Ok(())
}
