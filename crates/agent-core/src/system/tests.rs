use super::*;

#[test]
fn job_status_distinguishes_running_exited_failed_and_missing() -> Result<()> {
    for (properties, expected) in [
        ("LoadState=not-found\n", JobStatus::Missing),
        (
            "LoadState=loaded\nActiveState=activating\nSubState=start\n",
            JobStatus::Running,
        ),
        (
            "LoadState=loaded\nActiveState=active\nSubState=exited\nResult=success\nExecMainStatus=0\nExecMainCode=1\nExecMainStartTimestampMonotonic=123\n",
            JobStatus::Succeeded,
        ),
    ] {
        assert_eq!(
            parse_job_status(&CommandOutput {
                success: true,
                stdout: properties.into(),
                stderr: String::new()
            })?,
            expected
        );
    }
    let failed = parse_job_status(&CommandOutput { success: true, stdout: "LoadState=loaded\nActiveState=failed\nResult=timeout\nExecMainCode=2\nExecMainStatus=15\n".into(), stderr: String::new() })?;
    assert!(matches!(failed, JobStatus::Failed { error } if error.contains("timeout")));
    let never_started = parse_job_status(&CommandOutput { success: true, stdout: "LoadState=loaded\nActiveState=inactive\nResult=success\nExecMainCode=0\nExecMainStatus=0\nExecMainStartTimestampMonotonic=0\n".into(), stderr: String::new() })?;
    assert!(matches!(never_started, JobStatus::Failed { .. }));
    assert!(
        parse_job_status(&CommandOutput {
            success: false,
            stdout: String::new(),
            stderr: "bus unavailable".into()
        })
        .is_err()
    );
    Ok(())
}

#[tokio::test]
async fn unsafe_unit_and_expansion_arguments_are_rejected_before_execution() -> Result<()> {
    let services = SystemServiceManager::new(Arc::new(SystemOps));
    let mut job = ServiceJob {
        unit: format!("sinan-diagnostic-{}.service", Uuid::new_v4()),
        program: "/usr/bin/true".into(),
        args: vec!["$HOME".into()],
        working_directory: "/tmp".into(),
        timeout_secs: 1,
    };
    assert!(
        services
            .start_job(&job)
            .await
            .unwrap_err()
            .to_string()
            .contains("expansion")
    );
    job.args.clear();
    job.unit = "arbitrary.service".into();
    assert!(
        services
            .start_job(&job)
            .await
            .unwrap_err()
            .to_string()
            .contains("unit")
    );
    assert!(services.job_status("--bad").await.is_err());
    Ok(())
}

#[tokio::test]
#[ignore = "requires Linux, a running systemd system manager, and root to create isolated transient services"]
async fn real_systemd_diagnostic_jobs_survive_manager_recreation_and_enforce_timeout() -> Result<()>
{
    ensure!(
        cfg!(target_os = "linux"),
        "this integration test requires Linux/systemd"
    );
    let privileged: Arc<dyn Privileged> = Arc::new(SystemOps);
    let services = SystemServiceManager::new(privileged.clone());
    let directory = std::env::temp_dir().join(format!("sinan-service-test-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&directory)?;
    let mut units = Vec::new();
    let result = async {
        for (program, args, timeout_secs, succeeds) in [
            ("/usr/bin/printf", vec!["fixture report".into()], 10, true),
            ("/usr/bin/false", Vec::new(), 10, false),
            (
                "/bin/sh",
                vec!["-c".into(), "sleep 30 & wait".into()],
                1,
                false,
            ),
        ] {
            let job = ServiceJob {
                unit: format!("sinan-diagnostic-{}.service", Uuid::new_v4()),
                program: program.into(),
                args,
                working_directory: directory.clone(),
                timeout_secs,
            };
            units.push(job.unit.clone());
            services.start_job(&job).await?;
            // A fresh manager observes the system-owned service without another start.
            let recovered = SystemServiceManager::new(privileged.clone());
            let status = tokio::time::timeout(Duration::from_secs(45), async {
                loop {
                    let status = recovered.job_status(&job.unit).await?;
                    if status != JobStatus::Running {
                        return Ok::<_, anyhow::Error>(status);
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            })
            .await??;
            if succeeds {
                assert_eq!(status, JobStatus::Succeeded);
            } else {
                assert!(matches!(status, JobStatus::Failed { .. }));
            }
            if timeout_secs == 1 {
                assert!(matches!(status, JobStatus::Failed { error } if error.contains("timeout")));
            }
            let properties = privileged
                .execute(
                    Path::new("systemctl"),
                    &[
                        "show".into(),
                        "--property=PrivateMounts,KillMode,TimeoutStartUSec".into(),
                        "--".into(),
                        job.unit.clone(),
                    ],
                )
                .await?;
            ensure!(properties.success, "cannot inspect diagnostic isolation");
            assert!(properties.stdout.contains("PrivateMounts=yes"));
            assert!(properties.stdout.contains("KillMode=control-group"));
            if timeout_secs == 1 {
                assert!(properties.stdout.contains("TimeoutStartUSec=1s"));
            }
            recovered.stop(&job.unit).await?;
        }
        Ok::<_, anyhow::Error>(())
    }
    .await;
    for unit in units {
        let _ = services.stop(&unit).await;
        let _ = privileged
            .execute(
                Path::new("systemctl"),
                &["reset-failed".into(), "--".into(), unit],
            )
            .await;
    }
    std::fs::remove_dir_all(directory)?;
    result
}
