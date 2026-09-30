use super::*;

impl DiagnosticWorker {
    pub(super) async fn observe(&self, checkpoint: &Checkpoint) -> Result<()> {
        let Checkpoint::Started {
            spec,
            service,
            started_at,
            plugin,
            start_error,
            expires_at,
            protection_stop_reason,
            ..
        } = checkpoint
        else {
            anyhow::bail!("only a started diagnostic can be observed");
        };
        let id = Uuid::parse_str(&spec.id)?;
        if self.cancellation_requested(id)? {
            return Ok(());
        }
        let (status, stop_reason) = self
            .status_with_memory_protection(&service.unit, protection_stop_reason.as_deref())
            .await?;
        let now = unix_time();
        if let Err(error) = self.capture_environment(checkpoint) {
            tracing::warn!(%id, %error, "execution environment capture failed; resource protection continues");
        }
        let deadline_reached = expires_at.is_some_and(|deadline| deadline <= now as i64);
        if protection_stop_reason.is_none()
            && let Some(reason) = &stop_reason
        {
            let mut stopping = checkpoint.clone();
            if let Checkpoint::Started {
                protection_stop_reason,
                ..
            } = &mut stopping
            {
                *protection_stop_reason = Some(reason.clone());
            }
            if let Err(error) = self.save(&stopping) {
                // Storage failure must not prevent the memory protection action.
                tracing::warn!(%id, %error, "cannot persist protection reason; stopping diagnostic anyway");
            }
        }
        if status == Some(JobStatus::Running)
            && stop_reason.is_none()
            && !deadline_reached
            && now.saturating_sub(*started_at) <= u64::from(spec.timeout_secs) + 60
        {
            // Reading a damaged output filesystem must not delay resource protection.
            if let Err(error) =
                tokio::time::timeout(Duration::from_secs(2), self.capture_sections(spec, plugin))
                    .await
                    .context("diagnostic chapter capture timed out")
                    .and_then(|result| result)
            {
                tracing::warn!(%id, %error, "diagnostic chapter capture failed; saved chapters are retained");
            }
            return Ok(());
        }
        if status == Some(JobStatus::Running) || stop_reason.is_some() {
            self.bounded(self.services.stop(&service.unit))
                .await
                .context("诊断保护停止未确认，将保留任务并继续重试")?;
            ensure!(
                !self.bounded(self.services.is_active(&service.unit)).await?
                    && self
                        .bounded(self.services.job_status(&service.unit))
                        .await?
                        != JobStatus::Running,
                "诊断仍有活动进程，保护停止未确认，将继续重试"
            );
        }
        if let Err(error) =
            tokio::time::timeout(Duration::from_secs(2), self.capture_sections(spec, plugin))
                .await
                .context("final diagnostic chapter capture timed out")
                .and_then(|result| result)
        {
            tracing::warn!(%id, %error, "final diagnostic chapter capture failed; saved chapters are retained");
        }
        let collected = if let Some(adapter) = self.adapters.get(plugin) {
            self.bounded(adapter.collect(spec)).await
        } else {
            Err(anyhow::anyhow!("diagnostic plugin is no longer registered"))
        };
        let report = collected
            .as_ref()
            .ok()
            .and_then(|output| output.as_ref())
            .filter(|output| !output.text.is_empty() && output.text.len() <= MAX_REPORT)
            .map(|output| DiagnosticReport {
                text: output.text.clone(),
                report_url: output.report_url.clone(),
            });
        let update = if let Some(reason) = stop_reason {
            failure(id, reason, report)
        } else {
            match status.context("diagnostic status is unavailable")? {
                JobStatus::Succeeded if report.is_some() => DiagnosticUpdate {
                    id,
                    status: DiagnosticStatus::Succeeded,
                    report,
                    error: None,
                },
                JobStatus::Succeeded => failure(
                    id,
                    collected
                        .err()
                        .map(|error| format!("collect diagnostic: {error}"))
                        .unwrap_or_else(|| "diagnostic completed without a valid report".into()),
                    report,
                ),
                JobStatus::Failed { error } => failure(id, error, report),
                JobStatus::Missing => failure(
                    id,
                    format!(
                        "diagnostic service is missing after restart or an uncertain start; task was not repeated{}",
                        start_error
                            .as_ref()
                            .map(|error| format!("; start error: {error}"))
                            .unwrap_or_default()
                    ),
                    report,
                ),
                JobStatus::Running => failure(
                    id,
                    if deadline_reached {
                        "diagnostic reached its absolute deadline"
                    } else {
                        "diagnostic exceeded its execution deadline"
                    }
                    .into(),
                    report,
                ),
            }
        };
        if self.cancellation_requested(id)? {
            return Ok(());
        }
        self.finish(update)?;
        if let Err(error) = self.bounded(self.services.stop(&service.unit)).await {
            tracing::warn!(%id, %error, "diagnostic service cleanup failed");
        }
        Ok(())
    }
}
