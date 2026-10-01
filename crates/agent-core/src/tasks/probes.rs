mod icmp;
#[cfg(test)]
mod scheduling_tests;

use crate::{SharedState, artifacts::PanelClient};
use anyhow::{Context, Result, ensure};
use sinan_adapter_sdk::Privileged;
use sinan_protocol::{
    ProbeBatch, ProbeKind, ProbeResult, ProbeSpec, TaskAck, now_timestamp, telemetry::now_millis,
};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{
    net::{TcpStream, lookup_host},
    sync::watch,
    task::{AbortHandle, JoinSet},
    time::{Instant, timeout},
};
use uuid::Uuid;

pub(super) async fn run(
    state: SharedState,
    ops: Arc<dyn Privileged>,
    clients: watch::Receiver<Option<Arc<PanelClient>>>,
    retirement: Arc<crate::retirement::Retirement>,
) -> Result<()> {
    tokio::try_join!(
        sample_loop(state.clone(), ops, retirement.clone()),
        synchronize(state, clients, retirement)
    )?;
    Ok(())
}

async fn synchronize(
    state: SharedState,
    clients: watch::Receiver<Option<Arc<PanelClient>>>,
    retirement: Arc<crate::retirement::Retirement>,
) -> Result<()> {
    let mut refreshed = Instant::now() - Duration::from_secs(31);
    loop {
        {
            let _guard = retirement.gate.read().await;
            if !retirement.requested() {
                let client = clients.borrow().clone();
                if let Some(client) = client {
                    if refreshed.elapsed() >= Duration::from_secs(30) {
                        refreshed = Instant::now();
                        match client
                            .get_json::<Vec<ProbeSpec>>("/api/agent/v1/probes")
                            .await
                        {
                            Ok(specs)
                                if !retirement.requested()
                                    && specs.len() <= 32
                                    && specs.iter().all(ProbeSpec::valid) =>
                            {
                                state
                                    .lock()
                                    .map_err(|_| anyhow::anyhow!("state lock poisoned"))?
                                    .set_json("probes:configuration", &(now_timestamp(), specs))?;
                            }
                            Ok(_) => tracing::warn!("panel provided invalid probes"),
                            Err(error) => {
                                tracing::warn!(%error,"probe configuration refresh failed")
                            }
                        }
                    }
                    if !retirement.requested()
                        && let Err(error) = upload(&state, &client).await
                    {
                        tracing::warn!(%error,"probe results retained for retry");
                    }
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

async fn sample_loop(
    state: SharedState,
    ops: Arc<dyn Privileged>,
    retirement: Arc<crate::retirement::Retirement>,
) -> Result<()> {
    let mut due = HashMap::<Uuid, Instant>::new();
    let mut configured = HashMap::<Uuid, ProbeSpec>::new();
    let mut running = HashMap::<Uuid, (ProbeSpec, AbortHandle)>::new();
    let mut tasks = JoinSet::<(ProbeSpec, Option<ProbeResult>)>::new();
    let mut tick = tokio::time::interval(Duration::from_secs(1));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            completed = tasks.join_next(), if !tasks.is_empty() => {
                let Some(completed) = completed else { continue };
                match completed {
                    Ok((spec, Some(mut result))) => {
                        if running.get(&spec.id).is_some_and(|(current, _)| current == &spec) { running.remove(&spec.id); }
                        let _guard = retirement.gate.read().await;
                        if retirement.requested() { continue; }
                        let mut state = state.lock().map_err(|_| anyhow::anyhow!("state lock poisoned"))?;
                        let configuration = state.get_json::<(i64, Vec<ProbeSpec>)>("probes:configuration")?;
                        if !configuration.is_some_and(|(fetched, specs)| fetched > now_timestamp() - 86400 && specs.contains(&spec)) {
                            continue;
                        }
                        result.sampled_at = result.sampled_at.saturating_add(state.get_json::<i64>("clock_offset_ms")?.unwrap_or(0));
                        state.save_probe_result(&result)?;
                    }
                    Ok((spec, None)) => { running.remove(&spec.id); }
                    Err(error) if error.is_cancelled() => {}
                    Err(error) => return Err(error.into()),
                }
            }
            _ = tick.tick() => {
                let configuration = state.lock().map_err(|_| anyhow::anyhow!("state lock poisoned"))?
                    .get_json::<(i64, Vec<ProbeSpec>)>("probes:configuration")?;
                let specs = configuration.filter(|(fetched, _)| !retirement.requested() && *fetched > now_timestamp() - 86400)
                    .map(|(_, specs)| specs).unwrap_or_default();
                running.retain(|_, (spec, task)| {
                    let keep = specs.iter().any(|current| current.enabled && current == spec);
                    if !keep { task.abort(); }
                    keep
                });
                due.retain(|id, _| specs.iter().any(|spec| spec.id == *id && spec.enabled && configured.get(id) == Some(spec)));
                configured = specs.iter().map(|spec| (spec.id, spec.clone())).collect();
                let now = Instant::now();
                let mut ready: Vec<_> = specs.into_iter().filter(|spec| spec.enabled && !running.contains_key(&spec.id)
                    && due.get(&spec.id).is_none_or(|next| *next <= now)).collect();
                // Oldest due targets run first; slow targets do not block completed results.
                ready.sort_by_key(|spec| (due.get(&spec.id).copied(), spec.id));
                for spec in ready.into_iter().take(4usize.saturating_sub(tasks.len())) {
                    due.insert(spec.id, now + Duration::from_secs(u64::from(spec.interval_secs)));
                    let ops = ops.clone();
                    let retirement = retirement.clone();
                    let saved = spec.clone();
                    let task = tasks.spawn(async move {
                        let _guard = retirement.gate.read().await;
                        let result = if retirement.requested() { None } else { Some(sample(&spec, ops.as_ref()).await) };
                        (spec, result)
                    });
                    running.insert(saved.id, (saved, task));
                }
            }
        }
    }
}

async fn upload(state: &SharedState, client: &PanelClient) -> Result<()> {
    let results = state
        .lock()
        .map_err(|_| anyhow::anyhow!("state lock poisoned"))?
        .probe_results()?;
    if results.is_empty() {
        return Ok(());
    }
    let batch = ProbeBatch { results };
    let ack: TaskAck = client
        .post_json("/api/agent/v1/probe-results", &batch)
        .await?;
    ensure!(
        ack.ids.len() <= 64
            && ack
                .ids
                .iter()
                .all(|id| batch.results.iter().any(|v| v.id == *id)),
        "invalid probe acknowledgment"
    );
    state
        .lock()
        .map_err(|_| anyhow::anyhow!("state lock poisoned"))?
        .acknowledge_probes(&ack.ids)?;
    Ok(())
}

async fn sample(spec: &ProbeSpec, ops: &dyn Privileged) -> ProbeResult {
    let mut result = ProbeResult {
        id: Uuid::new_v4(),
        probe_id: spec.id,
        sampled_at: now_millis(),
        latency_ms: None,
        loss_percent: 100.0,
        error: None,
    };
    match timeout(Duration::from_secs(12), measure(spec, ops))
        .await
        .context("probe exceeded its deadline")
        .and_then(|result| result)
    {
        Ok(measurement) => {
            result.loss_percent = f64::from(4 - measurement.received) * 25.0;
            result.latency_ms = measurement.latency_ms;
        }
        Err(error) => result.error = Some(format!("{error:#}").chars().take(256).collect()),
    }
    result.sampled_at = now_millis();
    result
}

async fn measure(spec: &ProbeSpec, ops: &dyn Privileged) -> Result<icmp::Measurement> {
    ensure!(spec.valid(), "invalid probe configuration");
    let addresses = timeout(
        Duration::from_secs(2),
        lookup_host((spec.target.as_str(), spec.port.unwrap_or(0))),
    )
    .await
    .context("probe DNS lookup timed out")?
    .context("probe DNS lookup failed")?;
    let address = addresses
        .into_iter()
        .find(|a| !a.ip().is_unspecified() && !a.ip().is_multicast())
        .ok_or_else(|| anyhow::anyhow!("probe resolved no unicast address"))?;
    match spec.kind {
        ProbeKind::Tcp => {
            let mut times = Vec::new();
            for _ in 0..4 {
                let started = Instant::now();
                if matches!(
                    timeout(Duration::from_secs(1), TcpStream::connect(address)).await,
                    Ok(Ok(_))
                ) {
                    times.push(started.elapsed().as_secs_f64() * 1000.0);
                }
            }
            Ok(icmp::Measurement {
                received: times.len() as u32,
                latency_ms: (!times.is_empty())
                    .then(|| times.iter().sum::<f64>() / times.len() as f64),
            })
        }
        ProbeKind::Icmp => icmp::measure(address.ip(), ops).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    #[ignore = "requires the platform ICMP tool and permission to send IPv4/IPv6 loopback echoes"]
    async fn icmp_measures_real_ipv4_and_ipv6_loopback() -> Result<()> {
        for address in ["127.0.0.1", "::1"] {
            let measurement = icmp::measure(address.parse()?, &crate::system::SystemOps).await?;
            assert_eq!(measurement.received, 4);
            assert!(measurement.latency_ms.is_some());
        }
        Ok(())
    }
    #[tokio::test]
    async fn tcp_probe_measures_a_real_listener_and_closed_port() -> Result<()> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let mut spec = ProbeSpec {
            id: Uuid::new_v4(),
            name: "fixture".into(),
            kind: ProbeKind::Tcp,
            target: "127.0.0.1".into(),
            port: Some(listener.local_addr()?.port()),
            interval_secs: 10,
            carrier: String::new(),
            enabled: true,
        };
        let result = sample(&spec, &crate::system::SystemOps).await;
        assert_eq!(result.loss_percent, 0.0);
        assert!(result.latency_ms.is_some());
        // Reserve a distinct non-listening port so parallel tests cannot reuse it.
        let closed = tokio::net::TcpSocket::new_v4()?;
        closed.bind("127.0.0.1:0".parse()?)?;
        spec.port = Some(closed.local_addr()?.port());
        let failed = sample(&spec, &crate::system::SystemOps).await;
        assert_eq!(failed.loss_percent, 100.0);
        assert_eq!(failed.latency_ms, None);
        Ok(())
    }
    #[tokio::test]
    async fn invalid_target_is_unavailable_instead_of_a_loss_measurement() {
        let spec = ProbeSpec {
            id: Uuid::new_v4(),
            name: "fixture".into(),
            kind: ProbeKind::Icmp,
            target: "0.0.0.0".into(),
            port: None,
            interval_secs: 10,
            carrier: String::new(),
            enabled: true,
        };
        let result = sample(&spec, &crate::system::SystemOps).await;
        assert!(result.error.is_some());
        assert_eq!(result.latency_ms, None);
    }
}
