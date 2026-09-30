use super::*;

impl SystemServiceManager {
    pub(super) fn start_diagnostic_job<'a>(&'a self, job: &'a ServiceJob) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            ensure!(valid_job_unit(&job.unit), "invalid diagnostic service unit");
            ensure!(
                job.program.is_absolute()
                    && job.working_directory.is_absolute()
                    && (1..=3600).contains(&job.timeout_secs),
                "invalid diagnostic service configuration"
            );
            let program = job
                .program
                .to_str()
                .context("diagnostic program path is not UTF-8")?;
            let directory = job
                .working_directory
                .to_str()
                .context("diagnostic working directory is not UTF-8")?;
            ensure!(
                std::iter::once(program)
                    .chain(std::iter::once(directory))
                    .chain(job.args.iter().map(String::as_str))
                    .all(
                        |value| !value.chars().any(char::is_control) && !value.contains(['$', '%'])
                    ),
                "diagnostic arguments cannot contain systemd expansion syntax or control characters"
            );
            if self.backend == ServiceBackend::OpenRc {
                return self.start_openrc_job(job).await;
            }
            ensure!(
                self.backend == ServiceBackend::Systemd,
                "diagnostic jobs require Linux"
            );
            let mut args = vec![
                format!("--unit={}", job.unit),
                "--no-block".into(),
                "--property=Type=oneshot".into(),
                "--property=RemainAfterExit=yes".into(),
                format!("--property=TimeoutStartSec={}s", job.timeout_secs),
                "--property=TimeoutStopSec=30s".into(),
                "--property=KillMode=control-group".into(),
                "--property=PrivateMounts=yes".into(),
                "--property=UMask=0077".into(),
                "--property=StandardOutput=null".into(),
                "--property=StandardError=journal".into(),
                format!("--property=MemoryMax={}", job.memory_max.get()),
                "--property=MemorySwapMax=0".into(),
                format!("--property=TasksMax={}", job.tasks_max.get()),
                format!("--property=CPUWeight={}", job.cpu_weight.get()),
                format!("--property=IOWeight={}", job.io_weight.get()),
                format!("--property=OOMScoreAdjust={}", job.oom_score_adjust.get()),
                format!("--property=WorkingDirectory={directory}"),
                "--".into(),
                program.into(),
            ];
            args.extend(job.args.iter().cloned());
            let output = self
                .privileged
                .execute(Path::new("systemd-run"), &args)
                .await?;
            ensure!(
                output.success,
                "diagnostic service start failed: {}",
                output.stderr
            );
            Ok(())
        })
    }
    pub(super) fn diagnostic_job_status<'a>(&'a self, unit: &'a str) -> BoxFuture<'a, JobStatus> {
        Box::pin(async move {
            ensure!(valid_job_unit(unit), "invalid diagnostic service unit");
            if self.backend == ServiceBackend::OpenRc {
                return self.openrc_job_status(unit).await;
            }
            ensure!(
                self.backend == ServiceBackend::Systemd,
                "diagnostic jobs require Linux"
            );
            let args = vec!["show".into(), "--property=LoadState,ActiveState,SubState,Result,ExecMainCode,ExecMainStatus,ExecMainStartTimestampMonotonic,Job".into(), "--".into(), unit.into()];
            let output = self
                .privileged
                .execute(Path::new("systemctl"), &args)
                .await?;
            parse_job_status(&output)
        })
    }
}

fn valid_job_unit(unit: &str) -> bool {
    unit.strip_prefix("sinan-diagnostic-")
        .and_then(|value| value.strip_suffix(".service"))
        .is_some_and(|id| Uuid::parse_str(id).is_ok_and(|uuid| uuid.to_string() == id))
}

fn parse_job_status(output: &CommandOutput) -> Result<JobStatus> {
    let properties: std::collections::BTreeMap<_, _> = output
        .stdout
        .lines()
        .filter_map(|line| line.split_once('='))
        .collect();
    if properties.get("LoadState") == Some(&"not-found") {
        return Ok(JobStatus::Missing);
    }
    ensure!(
        output.success,
        "diagnostic service status query failed: {}",
        output.stderr
    );
    ensure!(
        properties.get("LoadState") == Some(&"loaded"),
        "diagnostic service load state is unknown"
    );
    match properties.get("ActiveState").copied() {
        Some("activating" | "deactivating" | "reloading") => Ok(JobStatus::Running),
        Some("active") if properties.get("SubState") != Some(&"exited") => Ok(JobStatus::Running),
        // --no-block can leave a start job queued while the unit is inactive.
        // Any outstanding job must settle before interpreting the last result.
        Some("active" | "inactive" | "failed")
            if properties.get("Job").is_some_and(|value| {
                value
                    .parse::<std::num::NonZeroU32>()
                    .is_ok_and(|id| id.to_string().as_str() == *value)
            }) =>
        {
            Ok(JobStatus::Running)
        }
        Some("active" | "inactive")
            if properties.get("Result") == Some(&"success")
                && properties.get("ExecMainStatus") == Some(&"0")
                && properties.get("ExecMainCode") == Some(&"1")
                && properties
                    .get("ExecMainStartTimestampMonotonic")
                    .is_some_and(|value| *value != "0" && !value.is_empty()) =>
        {
            Ok(JobStatus::Succeeded)
        }
        Some("failed" | "inactive" | "active") => Ok(JobStatus::Failed {
            error: format!(
                "diagnostic service failed: result={}, code={}, status={}",
                properties.get("Result").unwrap_or(&"unknown"),
                properties.get("ExecMainCode").unwrap_or(&"unknown"),
                properties.get("ExecMainStatus").unwrap_or(&"unknown")
            ),
        }),
        _ => anyhow::bail!("diagnostic service active state is unknown"),
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
