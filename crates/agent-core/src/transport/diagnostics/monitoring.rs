use super::*;

impl DiagnosticWorker {
    pub(super) async fn monitored_tick(&self, client: Option<&PanelClient>) -> Result<()> {
        if let Some(checkpoint @ Checkpoint::Started { .. }) = self.active()? {
            self.observe(&checkpoint).await?;
        }
        let Some(client) = client else {
            return Ok(());
        };
        if !matches!(self.active()?, Some(Checkpoint::Started { .. })) {
            return self.connected_tick(Some(client)).await;
        }
        // Only sending existing results/status can overlap protection. Preparation and
        // durable start remain sequential so an uncertain start cannot be repeated.
        let send = async {
            self.flush(client).await?;
            if let Some(Checkpoint::Started { spec, .. }) = self.active()? {
                self.bounded(client.diagnostic_update(&DiagnosticUpdate {
                    id: Uuid::parse_str(&spec.id)?,
                    status: DiagnosticStatus::Running,
                    report: None,
                    error: None,
                }))
                .await?;
            }
            Ok(())
        };
        tokio::pin!(send);
        let interval = Duration::from_secs(5);
        let mut protection =
            tokio::time::interval_at(tokio::time::Instant::now() + interval, interval);
        protection.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                result = &mut send => return result,
                _ = protection.tick() => {
                    if let Some(checkpoint @ Checkpoint::Started { .. }) = self.active()?
                        && let Err(error) = self.observe(&checkpoint).await {
                        tracing::warn!(%error, "diagnostic protection check failed; durable work will be retried");
                    }
                }
            }
        }
    }

    pub(crate) async fn run_guarded(
        self,
        mut client: watch::Receiver<Option<Arc<PanelClient>>>,
        retirement: Arc<crate::retirement::Retirement>,
    ) -> Result<()> {
        let mut poll = tokio::time::interval(Duration::from_secs(5));
        poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = poll.tick() => {},
                changed = client.changed() => { if changed.is_err() { return Ok(()); } },
            }
            let _guard = retirement.gate.read().await;
            if retirement.requested() {
                continue;
            }
            let active_client = client.borrow().clone();
            if let Err(error) = self.monitored_tick(active_client.as_deref()).await {
                tracing::warn!(%error, "diagnostic poll failed; durable work will be retried");
            }
        }
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
            if let Err(error) = self.monitored_tick(active_client.as_deref()).await {
                tracing::warn!(%error, "diagnostic poll failed; durable work will be retried");
            }
        }
    }
}
