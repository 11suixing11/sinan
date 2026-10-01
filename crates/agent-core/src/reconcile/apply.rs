use super::{ApplyIntent, Reconciler};
use crate::state::IntentRecord;
use anyhow::{Context, Result};
use sinan_adapter_sdk::{Plan, Prepared};
use sinan_protocol::{ApplyResult, ApplyStatus, UsageBatch, now_timestamp};
use uuid::Uuid;

impl Reconciler {
    /// Applies a target after independently verifying its locally signed executable.
    pub async fn apply_prepared(&self, target: Prepared) -> Result<ApplyResult> {
        {
            let mut pending = self
                .pending_prepared
                .lock()
                .map_err(|_| anyhow::anyhow!("pending target poisoned"))?;
            if pending
                .as_ref()
                .is_none_or(|current| target.spec.revision >= current.spec.revision)
            {
                *pending = Some(target);
            }
        }
        let _guard = self.gate.lock().await;
        let next = self
            .pending_prepared
            .lock()
            .map_err(|_| anyhow::anyhow!("pending target poisoned"))?
            .take();
        let Some(target) = next else {
            return self.current_result().await;
        };
        self.recover_locked().await?;
        let previous = self.previous()?;
        let op_id = Uuid::new_v4();
        let module = self.adapter.describe().module;
        let rev = target.spec.revision;
        match self.apply_prepared_locked(previous, target, op_id).await {
            Ok(()) => Ok(ApplyResult {
                module,
                rev,
                op_id,
                status: ApplyStatus::Applied,
                healthy: true,
                error: None,
            }),
            Err(error) => Ok(ApplyResult {
                module,
                rev,
                op_id,
                status: ApplyStatus::Failed,
                healthy: self
                    .state
                    .lock()
                    .map_err(|_| anyhow::anyhow!("state poisoned"))?
                    .get_json::<bool>(&format!("health:{}", self.adapter.describe().module))?
                    .unwrap_or(false),
                error: Some(format!("{error:#}")),
            }),
        }
    }

    pub(super) async fn current_result(&self) -> Result<ApplyResult> {
        let previous = self
            .previous()?
            .context("no applied runtime after coalescing")?;
        let module = self.adapter.describe().module;
        self.verify_applied_runtime(&previous).await?;
        let healthy = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("state poisoned"))?
            .get_json::<bool>(&format!("health:{module}"))?
            .unwrap_or(false);
        Ok(ApplyResult {
            module,
            rev: previous.spec.revision,
            op_id: Uuid::new_v4(),
            status: if healthy {
                ApplyStatus::Applied
            } else {
                ApplyStatus::Failed
            },
            healthy,
            error: (!healthy).then(|| "current runtime is unhealthy".into()),
        })
    }

    pub(super) async fn apply_prepared_locked(
        &self,
        previous: Option<Prepared>,
        target: Prepared,
        op_id: Uuid,
    ) -> Result<()> {
        self.verify_runtime(&target).await?;
        if let Some(previous) = &previous {
            self.verify_applied_runtime(previous).await?;
        }
        let plan = self
            .bounded(self.adapter.plan(previous.as_ref(), &target))
            .await?;
        self.apply_plan_locked(previous, target, plan, op_id).await
    }

    pub(super) async fn apply_plan_locked(
        &self,
        previous: Option<Prepared>,
        target: Prepared,
        plan: Plan,
        op_id: Uuid,
    ) -> Result<()> {
        let descriptor = self.adapter.describe();
        self.verify_runtime(&target).await?;
        if let Some(previous) = &previous {
            self.verify_applied_runtime(previous).await?;
        }
        let intent = ApplyIntent {
            previous,
            target,
            plan,
        };
        self.state
            .lock()
            .map_err(|_| anyhow::anyhow!("state poisoned"))?
            .begin_intent(&IntentRecord {
                op_id,
                module: descriptor.module.clone(),
                payload: serde_json::to_value(&intent)?,
            })?;
        if plan != Plan::Noop
            && let Some(previous) = &intent.previous
            && let Err(error) = self.sample_runtime(previous).await
        {
            self.state
                .lock()
                .map_err(|_| anyhow::anyhow!("state poisoned"))?
                .finish_intent(op_id)?;
            return Err(error.context("cannot capture terminal counters; application cancelled"));
        }
        let attempt: Result<()> = async {
            self.switch(&intent.target).await?;
            self.bounded(
                self.adapter
                    .apply(plan, &intent.target, self.services.as_ref()),
            )
            .await?;
            if plan != Plan::Noop {
                self.state
                    .lock()
                    .map_err(|_| anyhow::anyhow!("state poisoned"))?
                    .begin_usage_epoch(&descriptor.module, now_timestamp())?;
            }
            anyhow::ensure!(
                self.runtime_health(&intent.target).await?,
                "runtime failed health check"
            );
            self.checkpoint(op_id, Some(&intent.target), true)?;
            Ok(())
        }
        .await;
        if let Err(error) = attempt {
            if let Err(rollback) = self.rollback(&intent, op_id).await {
                self.state
                    .lock()
                    .map_err(|_| anyhow::anyhow!("state poisoned"))?
                    .set_json(&format!("health:{}", descriptor.module), &false)?;
                return Err(error.context(format!("rollback also failed: {rollback:#}")));
            }
            return Err(error);
        }
        Ok(())
    }

    async fn switch(&self, target: &Prepared) -> Result<()> {
        self.verify_runtime(target).await?;
        let descriptor = self.adapter.describe();
        let kernel_link = self
            .config
            .install_root
            .join(&descriptor.plugin_name)
            .join("current");
        let kernel_directory = target
            .spec
            .binary_path
            .parent()
            .context("binary has no parent")?;
        self.bounded(
            self.privileged
                .atomic_symlink(&kernel_link, kernel_directory),
        )
        .await?;
        let current = self
            .config
            .runtime_root
            .join(format!("{}@main", descriptor.plugin_name))
            .join("current");
        self.bounded(
            self.privileged
                .atomic_symlink(&current, &target.spec.revision_dir),
        )
        .await
    }

    pub(super) async fn rollback(&self, intent: &ApplyIntent, op_id: Uuid) -> Result<()> {
        let descriptor = self.adapter.describe();
        if let Some(previous) = &intent.previous {
            self.verify_applied_runtime(previous).await?;
        }
        if let Err(error) = self.sample_runtime(&intent.target).await {
            tracing::warn!(error=%error,"cannot capture counters during recovery; possible missing window");
        }
        if let Some(previous) = &intent.previous {
            self.switch(previous).await?;
            self.bounded(
                self.adapter
                    .apply(Plan::Restart, previous, self.services.as_ref()),
            )
            .await?;
            self.state
                .lock()
                .map_err(|_| anyhow::anyhow!("state poisoned"))?
                .begin_usage_epoch(&descriptor.module, now_timestamp())?;
            anyhow::ensure!(
                self.runtime_health(previous).await?,
                "rollback health check failed"
            );
            self.checkpoint(op_id, Some(previous), true)?;
        } else {
            self.bounded(self.services.stop(&descriptor.service_unit))
                .await?;
            let runtime_link = self
                .config
                .runtime_root
                .join(format!("{}@main", descriptor.plugin_name))
                .join("current");
            self.bounded(self.privileged.remove_symlink(&runtime_link))
                .await?;
            let kernel_link = self
                .config
                .install_root
                .join(&descriptor.plugin_name)
                .join("current");
            self.bounded(self.privileged.remove_symlink(&kernel_link))
                .await?;
            self.checkpoint(op_id, None, false)?;
        }
        Ok(())
    }

    fn checkpoint(&self, op_id: Uuid, applied: Option<&Prepared>, healthy: bool) -> Result<()> {
        let module = self.adapter.describe().module;
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("state poisoned"))?;
        let mut updates = vec![(format!("health:{module}"), serde_json::json!(healthy))];
        if let Some(applied) = applied {
            updates.push((format!("applied:{module}"), serde_json::to_value(applied)?));
        } else {
            state.remove_json(&format!("applied:{module}"))?;
        }
        state.complete_intent(op_id, &updates)
    }

    async fn sample_runtime(&self, runtime: &Prepared) -> Result<Option<UsageBatch>> {
        let Some(source) = self.adapter.usage_source() else {
            return Ok(None);
        };
        let counters = self.bounded(source.read_counters(runtime)).await?;
        self.state
            .lock()
            .map_err(|_| anyhow::anyhow!("state poisoned"))?
            .sample_usage(&self.adapter.describe().module, &counters, now_timestamp())
    }

    pub async fn sample_usage(&self) -> Result<Option<UsageBatch>> {
        let _guard = self.gate.lock().await;
        if let Some(previous) = self.previous()? {
            self.sample_runtime(&previous).await
        } else {
            Ok(None)
        }
    }
}
