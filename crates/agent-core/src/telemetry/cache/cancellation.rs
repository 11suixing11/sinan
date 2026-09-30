use anyhow::{Result, ensure};
use std::{
    future::Future,
    sync::{Condvar, Mutex},
    time::Duration,
};
use tokio::{runtime::Runtime, sync::watch};

pub(super) struct Cancellation {
    signal: watch::Sender<bool>,
    active: Mutex<bool>,
    cleaned: Condvar,
}

impl Default for Cancellation {
    fn default() -> Self {
        Self {
            signal: watch::channel(false).0,
            active: Mutex::new(false),
            cleaned: Condvar::new(),
        }
    }
}

impl Cancellation {
    pub fn requested(&self) -> bool {
        *self.signal.borrow()
    }

    pub fn run<T>(
        &self,
        runtime: &Runtime,
        operation: impl Future<Output = Result<T>>,
    ) -> Result<T> {
        {
            let mut active = self
                .active
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            // Serialize asynchronous discovery admission with shutdown. A late
            // collector cannot enter another discovery after its owner returns.
            ensure!(!self.requested(), "telemetry collection stopped");
            *active = true;
        }
        let _cleanup = Activity(self);
        let mut signal = self.signal.subscribe();
        runtime.block_on(async {
            tokio::select! {
                biased;
                _ = async {
                    while !*signal.borrow_and_update() {
                        if signal.changed().await.is_err() { break; }
                    }
                } => anyhow::bail!("telemetry collection stopped"),
                result = operation => result,
            }
        })
        // Activity is released only after the cancelled operation and its
        // process-group guard have been dropped by select.
    }

    pub fn stop(&self) {
        let active = self
            .active
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        self.signal.send_replace(true);
        if !*active {
            return;
        }
        // A synchronous filesystem poll inside hardware discovery may itself be
        // stuck. Wait only for the asynchronous command cleanup, never join the
        // collector or indefinitely wait for a kernel read.
        let (active, timeout) = self
            .cleaned
            .wait_timeout_while(active, Duration::from_secs(1), |active| *active)
            .unwrap_or_else(|error| error.into_inner());
        if timeout.timed_out() && *active {
            tracing::warn!("telemetry cleanup confirmation timed out; collection remains stopped");
        }
    }

    #[cfg(test)]
    pub fn is_active(&self) -> bool {
        *self
            .active
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }
}

struct Activity<'a>(&'a Cancellation);

impl Drop for Activity<'_> {
    fn drop(&mut self) {
        let mut active = self
            .0
            .active
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *active = false;
        self.0.cleaned.notify_all();
    }
}
