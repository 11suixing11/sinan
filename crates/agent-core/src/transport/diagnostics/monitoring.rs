use super::*;

impl DiagnosticWorker {
    pub(super) async fn monitored_tick(&self, client: Option<&PanelClient>) -> Result<()> {
        self.process_cancellations().await?;
        if self.retiring() {
            return Ok(());
        }
        let Some(control) = &self.cancellations else {
            return self.network_tick(client).await;
        };
        let sending_existing = matches!(self.active()?, Some(Checkpoint::Started { .. }));
        let mut wake = control.subscribe();
        let outcome = {
            let work = self.network_tick(client);
            tokio::pin!(work);
            loop {
                tokio::select! {
                    result = &mut work => break Some(result),
                    changed = wake.changed() => {
                        changed.context("cancellation wake channel closed")?;
                        let interrupt = match self.active()? {
                            Some(Checkpoint::Preparing(job)) => control.requested(job.id)?,
                            None => true,
                            Some(Checkpoint::Started { .. }) => false,
                        };
                        if interrupt { break None; }
                        // Once a start request is in flight, settle it before checking
                        // cleanup. Preparing downloads can be dropped before that point.
                        if sending_existing { self.process_cancellations().await?; }
                    }
                }
            }
        };
        self.process_cancellations().await?;
        outcome.unwrap_or(Ok(()))
    }

    async fn network_tick(&self, client: Option<&PanelClient>) -> Result<()> {
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
                    self.process_cancellations().await?;
                    if let Some(checkpoint @ Checkpoint::Started { .. }) = self.active()?
                        && let Err(error) = self.observe(&checkpoint).await {
                        tracing::warn!(%error, "diagnostic protection check failed; durable work will be retried");
                    }
                }
            }
        }
    }

    pub(crate) async fn run_guarded(
        mut self,
        mut client: watch::Receiver<Option<Arc<PanelClient>>>,
        retirement: Arc<crate::retirement::Retirement>,
    ) -> Result<()> {
        self.retirement = Some(retirement.clone());
        let mut cancellation_wake = self
            .cancellations
            .as_ref()
            .map(|control| control.subscribe());
        let mut poll = tokio::time::interval(Duration::from_secs(5));
        poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = poll.tick() => {},
                _ = async {
                    if let Some(wake) = &mut cancellation_wake { let _ = wake.changed().await; }
                    else { std::future::pending::<()>().await; }
                } => {},
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
        let mut cancellation_wake = self
            .cancellations
            .as_ref()
            .map(|control| control.subscribe());
        let mut poll = tokio::time::interval(Duration::from_secs(5));
        poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = poll.tick() => {},
                _ = async {
                    if let Some(wake) = &mut cancellation_wake { let _ = wake.changed().await; }
                    else { std::future::pending::<()>().await; }
                } => {},
                changed = client.changed() => { if changed.is_err() { return Ok(()); } },
            }
            let active_client = client.borrow().clone();
            if let Err(error) = self.monitored_tick(active_client.as_deref()).await {
                tracing::warn!(%error, "diagnostic poll failed; durable work will be retried");
            }
        }
    }
}
