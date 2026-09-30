use crate::{
    Config, SharedState,
    artifacts::{PanelClient, safe_component},
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sinan_adapter_sdk::{
    Descriptor, DiagnosticAdapter, DiagnosticSpec, JobStatus, Privileged, ServiceJob,
    ServiceManager,
};
use sinan_protocol::release::{ReleaseError, TrustedKeys};
use sinan_protocol::{DiagnosticJob, DiagnosticReport, DiagnosticStatus, DiagnosticUpdate};
use std::{
    collections::BTreeMap,
    future::Future,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::watch;
use uuid::Uuid;

const ACTIVE: &str = "diagnostics:active";
const OUTBOX: &str = "diagnostics:outbox";
const MAX_REPORT: usize = 512 * 1024;

#[derive(Clone, Serialize, Deserialize)]
enum Checkpoint {
    Preparing(DiagnosticJob),
    Started {
        spec: DiagnosticSpec,
        service: ServiceJob,
        started_at: u64,
        plugin: String,
        #[serde(default)]
        start_error: Option<String>,
        #[serde(default)]
        expires_at: Option<i64>,
    },
}

pub struct DiagnosticWorker {
    config: Config,
    state: SharedState,
    adapters: BTreeMap<String, Arc<dyn DiagnosticAdapter>>,
    privileged: Arc<dyn Privileged>,
    services: Arc<dyn ServiceManager>,
    keys: std::result::Result<TrustedKeys, ReleaseError>,
}

impl DiagnosticWorker {
    pub fn new(
        config: Config,
        state: SharedState,
        adapters: Vec<Arc<dyn DiagnosticAdapter>>,
        privileged: Arc<dyn Privileged>,
        services: Arc<dyn ServiceManager>,
    ) -> Result<Self> {
        let mut registered = BTreeMap::new();
        for adapter in adapters {
            let descriptor = adapter.describe();
            ensure!(
                safe_component(&descriptor.plugin_name) && safe_component(&descriptor.binary_name),
                "invalid diagnostic plugin descriptor"
            );
            ensure!(
                registered.insert(descriptor.plugin_name, adapter).is_none(),
                "duplicate diagnostic plugin"
            );
        }
        Ok(Self {
            config,
            state,
            adapters: registered,
            privileged,
            services,
            keys: TrustedKeys::compiled(),
        })
    }

    /// Supplies roots already trusted by an embedding caller, including test fixtures.
    pub fn with_trusted_keys(mut self, keys: TrustedKeys) -> Self {
        self.keys = Ok(keys);
        self
    }

    fn read<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        self.state
            .lock()
            .map_err(|_| anyhow::anyhow!("state lock poisoned"))?
            .get_json(key)
    }

    async fn bounded<T>(&self, future: impl Future<Output = Result<T>>) -> Result<T> {
        tokio::time::timeout(
            Duration::from_secs(self.config.operation_timeout_secs),
            future,
        )
        .await
        .context("diagnostic operation timed out")?
    }

    fn save(&self, checkpoint: &Checkpoint) -> Result<()> {
        self.state
            .lock()
            .map_err(|_| anyhow::anyhow!("state lock poisoned"))?
            .set_json(ACTIVE, &Some(checkpoint))
    }

    fn active(&self) -> Result<Option<Checkpoint>> {
        Ok(self.read::<Option<Checkpoint>>(ACTIVE)?.flatten())
    }

    fn finish(&self, update: DiagnosticUpdate) -> Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("state lock poisoned"))?;
        let mut outbox: Vec<DiagnosticUpdate> = state.get_json(OUTBOX)?.unwrap_or_default();
        if !outbox.iter().any(|saved| saved.id == update.id) {
            outbox.push(update.clone());
        }
        state.set_json_batch(&[
            (ACTIVE.into(), serde_json::Value::Null),
            (OUTBOX.into(), serde_json::to_value(outbox)?),
            (
                format!("diagnostics:done:{}", update.id),
                serde_json::Value::Bool(true),
            ),
        ])
    }

    async fn flush(&self, client: &PanelClient) -> Result<()> {
        let pending: Vec<DiagnosticUpdate> = self.read(OUTBOX)?.unwrap_or_default();
        for update in pending {
            self.bounded(client.diagnostic_update(&update)).await?;
            let mut state = self
                .state
                .lock()
                .map_err(|_| anyhow::anyhow!("state lock poisoned"))?;
            let mut outbox: Vec<DiagnosticUpdate> = state.get_json(OUTBOX)?.unwrap_or_default();
            outbox.retain(|saved| saved.id != update.id);
            state.set_json(OUTBOX, &outbox)?;
        }
        Ok(())
    }

    fn accept(&self, jobs: Vec<DiagnosticJob>) -> Result<()> {
        if self.active()?.is_some() {
            return Ok(());
        }
        for job in jobs {
            if self
                .read::<bool>(&format!("diagnostics:done:{}", job.id))?
                .unwrap_or(false)
            {
                continue;
            }
            if !self.adapters.contains_key(&job.plugin)
                || !safe_component(&job.version)
                || !(1..=3600).contains(&job.timeout_secs)
                || expired(&job)
            {
                self.finish(failure(
                    job.id,
                    "unsupported diagnostic plugin, invalid job, or expired task".into(),
                    None,
                ))?;
                continue;
            }
            self.save(&Checkpoint::Preparing(job))?;
            break;
        }
        Ok(())
    }

    async fn prepare(&self, job: DiagnosticJob, client: &PanelClient) -> Result<()> {
        let id = job.id;
        let result: Result<()> = async {
            ensure!(!expired(&job), "diagnostic task has expired");
            let adapter = self.adapters.get(&job.plugin).context("diagnostic plugin is not registered")?;
            let descriptor = adapter.describe();
            let signed_descriptor = Descriptor {
                module: "diagnostics".into(), plugin_name: descriptor.plugin_name.clone(),
                binary_name: descriptor.binary_name.clone(), service_unit: String::new(), service_group: String::new(),
            };
            let binary = tokio::time::timeout(Duration::from_secs(300), client.ensure_artifact(&job.artifact, &job.version, &Descriptor {
                module: "diagnostics".into(), plugin_name: descriptor.plugin_name,
                binary_name: descriptor.binary_name, service_unit: String::new(), service_group: String::new(),
            }, &self.config.install_root, self.privileged.as_ref())).await.context("diagnostic artifact installation timed out")??;
            ensure!(!expired(&job), "diagnostic task expired during artifact installation");
            let keys = self.keys.as_ref().map_err(|error| anyhow::anyhow!(error.to_string()))?;
            crate::artifacts::verify_expected(&binary, &signed_descriptor, &job.version, keys).await?;
            let mut spec = DiagnosticSpec {
                id: id.to_string(), version: job.version,
                binary_path: binary,
                job_dir: self.config.runtime_root.join("diagnostics").join(id.to_string()),
                timeout_secs: execution_timeout(job.timeout_secs, job.expires_at, unix_time())?, options: job.options,
            };
            let mut service = self.bounded(adapter.prepare(&spec, self.privileged.as_ref())).await?;
            ensure!(service.unit == format!("sinan-diagnostic-{id}.service")
                && service.timeout_secs == spec.timeout_secs
                && service.working_directory == spec.job_dir && service.program == spec.binary_path, "invalid prepared diagnostic service");
            let started_at = unix_time();
            // Preparation can consume part of the remaining budget. Recompute immediately
            // before the durable start checkpoint and give systemd the reduced limit.
            let timeout_secs = execution_timeout(job.timeout_secs, job.expires_at, started_at)?;
            spec.timeout_secs = timeout_secs;
            service.timeout_secs = timeout_secs;
            crate::artifacts::verify_expected(&spec.binary_path, &signed_descriptor, &spec.version, keys).await?;
            // This checkpoint is durable before asking systemd to start anything. Recovery
            // only observes it; an uncertain start must never execute the task again.
            let mut checkpoint = Checkpoint::Started {
                spec, service: service.clone(), started_at, plugin: job.plugin, start_error: None, expires_at: job.expires_at,
            };
            self.save(&checkpoint)?;
            if let Err(error) = self.bounded(self.services.start_job(&service)).await {
                tracing::warn!(%id, %error, "diagnostic start response is uncertain; inspecting service on next poll");
                if let Checkpoint::Started { start_error, .. } = &mut checkpoint {
                    *start_error = Some(error.to_string().chars().take(4096).collect());
                }
                self.save(&checkpoint)?;
            }
            Ok(())
        }.await;
        if let Err(error) = result {
            self.finish(failure(id, format!("prepare diagnostic: {error}"), None))?;
        }
        Ok(())
    }

    async fn observe(&self, checkpoint: &Checkpoint) -> Result<()> {
        let Checkpoint::Started {
            spec,
            service,
            started_at,
            plugin,
            start_error,
            expires_at,
        } = checkpoint
        else {
            anyhow::bail!("only a started diagnostic can be observed");
        };
        let status = self
            .bounded(self.services.job_status(&service.unit))
            .await?;
        let id = Uuid::parse_str(&spec.id)?;
        let now = unix_time();
        let deadline_reached = expires_at.is_some_and(|deadline| deadline <= now as i64);
        if status == JobStatus::Running
            && !deadline_reached
            && now.saturating_sub(*started_at) <= u64::from(spec.timeout_secs) + 60
        {
            return Ok(());
        }
        if status == JobStatus::Running {
            self.bounded(self.services.stop(&service.unit)).await?;
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
        let update = match status {
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
        };
        self.finish(update)?;
        if let Err(error) = self.bounded(self.services.stop(&service.unit)).await {
            tracing::warn!(%id, %error, "diagnostic service cleanup failed");
        }
        Ok(())
    }

    pub async fn tick(&self, client: Option<&PanelClient>) -> Result<()> {
        if let Some(checkpoint @ Checkpoint::Started { .. }) = self.active()? {
            self.observe(&checkpoint).await?;
        }
        let Some(client) = client else {
            return Ok(());
        };
        // Pending terminal results are retried before another task is accepted.
        self.flush(client).await?;
        if self.active()?.is_none() {
            self.accept(self.bounded(client.diagnostic_jobs()).await?)?;
        }
        match self.active()? {
            Some(Checkpoint::Preparing(job)) => self.prepare(job, client).await?,
            Some(Checkpoint::Started { spec, .. }) => {
                self.bounded(client.diagnostic_update(&DiagnosticUpdate {
                    id: Uuid::parse_str(&spec.id)?,
                    status: DiagnosticStatus::Running,
                    report: None,
                    error: None,
                }))
                .await?
            }
            None => {}
        }
        Ok(())
    }

    pub async fn run(self, mut client: watch::Receiver<Option<Arc<PanelClient>>>) -> Result<()> {
        let mut poll = tokio::time::interval(Duration::from_secs(5));
        poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = poll.tick() => {},
                changed = client.changed() => { if changed.is_err() { return Ok(()); } },
            }
            let active_client = client.borrow().clone();
            if let Err(error) = self.tick(active_client.as_deref()).await {
                tracing::warn!(%error, "diagnostic poll failed; durable work will be retried");
            }
        }
    }
}

fn unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn expired(job: &DiagnosticJob) -> bool {
    job.expires_at
        .is_some_and(|deadline| deadline <= unix_time() as i64)
}

fn execution_timeout(configured: u64, expires_at: Option<i64>, now: u64) -> Result<u32> {
    let remaining = match expires_at {
        Some(deadline) => u64::try_from(deadline)
            .ok()
            .and_then(|deadline| deadline.checked_sub(now))
            .filter(|remaining| *remaining > 0)
            .context("diagnostic task expired before service start")?,
        None => configured,
    };
    ensure!(
        (1..=3600).contains(&configured),
        "invalid diagnostic timeout"
    );
    Ok(u32::try_from(configured.min(remaining))?)
}

fn failure(id: Uuid, error: String, report: Option<DiagnosticReport>) -> DiagnosticUpdate {
    DiagnosticUpdate {
        id,
        status: DiagnosticStatus::Failed,
        report,
        error: Some(error.chars().take(4096).collect()),
    }
}

#[cfg(test)]
#[path = "diagnostics/tests.rs"]
mod tests;
