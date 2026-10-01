mod icmp;
const CONFIGURATION_LEASE_SECS: i64 = 120;
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
    let mut configuration_storage = crate::state::StorageRetry::default();
    let mut acknowledgment_storage = crate::state::StorageRetry::default();
    loop {
        {
            let _guard = retirement.gate.read().await;
            if !retirement.requested() {
                let client = clients.borrow().clone();
                if let Some(client) = client {
                    if refreshed.elapsed() >= Duration::from_secs(30) {
                        refreshed = Instant::now();
                        match client
                            .get_json::<Vec<ProbeSpec>>("/api/agent/v1/probes/authorized")
                            .await
                        {
                            Ok(specs)
                                if !retirement.requested()
                                    && specs.len() <= 32
                                    && specs.iter().all(ProbeSpec::valid) =>
                            {
                                let mut state = state
                                    .lock()
                                    .map_err(|_| anyhow::anyhow!("state lock poisoned"))?;
                                configuration_storage.finish(
                                    "persist probe configuration",
                                    state.set_json(
                                        "probes:configuration",
                                        &(now_timestamp(), specs),
                                    ),
                                )?;
                            }
                            Ok(_) => tracing::warn!("panel provided invalid probes"),
                            Err(error) => {
                                tracing::warn!(%error,"probe configuration refresh failed")
                            }
                        }
                    }
                    if !retirement.requested()
                        && let Err(error) =
                            upload(&state, &client, &mut acknowledgment_storage).await
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
    let mut storage = crate::state::StorageRetry::default();
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
                        let configuration = leased_configuration(&state)?;
                        let clock_offset_ms = state.get_json::<i64>("clock_offset_ms")?.unwrap_or(0);
                        if !spec.runnable_at(panel_now_millis(clock_offset_ms).div_euclid(1000)) || !configuration.contains(&spec) {
                            continue;
                        }
                        result.sampled_at = result.sampled_at.saturating_add(state.get_json::<i64>("clock_offset_ms")?.unwrap_or(0));
                        storage.finish("persist probe sample", state.save_probe_result(&result))?;
                    }
                    Ok((spec, None)) => { running.remove(&spec.id); }
                    Err(error) if error.is_cancelled() => {}
                    Err(error) => return Err(error.into()),
                }
            }
            _ = tick.tick() => {
                let (configuration, clock_offset_ms) = {
                    let state = state.lock().map_err(|_| anyhow::anyhow!("state lock poisoned"))?;
                    (leased_configuration(&state)?,
                        state.get_json::<i64>("clock_offset_ms")?.unwrap_or(0))
                };
                let panel_now = panel_now_millis(clock_offset_ms).div_euclid(1000);
                let specs = if retirement.requested() { Vec::new() } else { configuration };
                running.retain(|_, (spec, task)| {
                    let keep = specs.iter().any(|current| current.runnable_at(panel_now) && current == spec);
                    if !keep { task.abort(); }
                    keep
                });
                due.retain(|id, _| specs.iter().any(|spec| spec.id == *id && spec.runnable_at(panel_now) && configured.get(id) == Some(spec)));
                configured = specs.iter().map(|spec| (spec.id, spec.clone())).collect();
                let now = Instant::now();
                let mut ready: Vec<_> = specs.into_iter().filter(|spec| spec.runnable_at(panel_now) && !running.contains_key(&spec.id)
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
                        let result = if retirement.requested() { None } else { Some(sample(&spec, ops.as_ref(), clock_offset_ms).await) };
                        (spec, result)
                    });
                    running.insert(saved.id, (saved, task));
                }
            }
        }
    }
}

async fn upload(
    state: &SharedState,
    client: &PanelClient,
    storage: &mut crate::state::StorageRetry,
) -> Result<()> {
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
    let mut state = state
        .lock()
        .map_err(|_| anyhow::anyhow!("state lock poisoned"))?;
    storage.finish(
        "persist probe acknowledgment",
        state.acknowledge_probes(&ack.ids),
    )?;
    Ok(())
}

fn leased_configuration(state: &crate::State) -> Result<Vec<ProbeSpec>> {
    let now = now_timestamp();
    let panel_now =
        panel_now_millis(state.get_json::<i64>("clock_offset_ms")?.unwrap_or(0)).div_euclid(1000);
    Ok(state
        .get_json::<(i64, Vec<ProbeSpec>)>("probes:configuration")?
        .filter(|(fetched, specs)| {
            *fetched <= now
                && now.saturating_sub(*fetched) < CONFIGURATION_LEASE_SECS
                && specs.len() <= 32
        })
        .map(|(_, specs)| {
            specs
                .into_iter()
                .filter(|spec| spec.runnable_at(panel_now))
                .collect()
        })
        .unwrap_or_default())
}

fn panel_now_millis(clock_offset_ms: i64) -> i64 {
    now_millis().saturating_add(clock_offset_ms)
}

async fn sample(spec: &ProbeSpec, ops: &dyn Privileged, clock_offset_ms: i64) -> ProbeResult {
    let mut result = ProbeResult {
        id: Uuid::new_v4(),
        probe_id: spec.id,
        sampled_at: now_millis(),
        latency_ms: None,
        loss_percent: 100.0,
        error: None,
        address_family: None,
        attempts: None,
    };
    let deadline = spec
        .monitor
        .as_ref()
        .and_then(|monitor| monitor.authorization.as_ref())
        .and_then(|authorization| authorization.expires_at)
        .map(|expires| {
            Duration::from_millis(
                expires
                    .saturating_mul(1000)
                    .saturating_sub(panel_now_millis(clock_offset_ms))
                    .max(0) as u64,
            )
        })
        .unwrap_or(Duration::from_secs(12))
        .min(Duration::from_secs(12));
    match timeout(deadline, measure(spec, ops, clock_offset_ms, &mut result))
        .await
        .context("probe exceeded its deadline")
        .and_then(|result| result)
    {
        Ok(measurement) => {
            result.attempts = Some(4);
            result.loss_percent = f64::from(4 - measurement.received) * 25.0;
            result.latency_ms = measurement.latency_ms;
        }
        Err(error) => result.error = Some(format!("{error:#}").chars().take(256).collect()),
    }
    result.sampled_at = now_millis();
    result
}

async fn measure(
    spec: &ProbeSpec,
    ops: &dyn Privileged,
    clock_offset_ms: i64,
    result: &mut ProbeResult,
) -> Result<icmp::Measurement> {
    ensure!(
        spec.runnable_at(panel_now_millis(clock_offset_ms).div_euclid(1000)),
        "probe target is not authorized or enabled"
    );
    let addresses = timeout(
        Duration::from_secs(2),
        lookup_host((spec.target.as_str(), spec.port.unwrap_or(0))),
    )
    .await
    .context("probe DNS lookup timed out")?
    .context("probe DNS lookup failed")?;
    ensure!(
        spec.runnable_at(panel_now_millis(clock_offset_ms).div_euclid(1000)),
        "probe authorization expired during DNS lookup"
    );
    let address = addresses
        .into_iter()
        .find(|a| {
            spec.address_family().allows(a.ip())
                && !a.ip().is_unspecified()
                && !a.ip().is_multicast()
        })
        .ok_or_else(|| anyhow::anyhow!("probe resolved no authorized-family unicast address"))?;
    result.address_family = Some(if address.is_ipv4() {
        sinan_protocol::ProbeAddressFamily::Ipv4
    } else {
        sinan_protocol::ProbeAddressFamily::Ipv6
    });
    match spec.kind {
        ProbeKind::Tcp => {
            let mut times = Vec::new();
            let mut last_error = None;
            for _ in 0..4 {
                ensure!(
                    spec.runnable_at(panel_now_millis(clock_offset_ms).div_euclid(1000)),
                    "probe authorization expired"
                );
                let started = Instant::now();
                match timeout(Duration::from_secs(1), TcpStream::connect(address)).await {
                    Ok(Ok(_)) => times.push(started.elapsed().as_secs_f64() * 1000.0),
                    Ok(Err(error)) => last_error = Some(format!("TCP connection failed: {error}")),
                    Err(_) => last_error = Some("TCP connection timed out after 1 second".into()),
                }
            }
            result.error = last_error.map(|error| error.chars().take(256).collect());
            Ok(icmp::Measurement {
                received: times.len() as u32,
                latency_ms: (!times.is_empty())
                    .then(|| times.iter().sum::<f64>() / times.len() as f64),
            })
        }
        ProbeKind::Icmp => {
            ensure!(
                spec.runnable_at(panel_now_millis(clock_offset_ms).div_euclid(1000)),
                "probe authorization expired before ICMP execution"
            );
            icmp::measure(address.ip(), ops).await
        }
    }
}

#[cfg(test)]
fn authorize_fixture(spec: &mut ProbeSpec) {
    let identity = spec.identity();
    spec.monitor = Some(sinan_protocol::ProbeMonitor {
        network: sinan_protocol::ProbeNetwork::Other,
        region: String::new(),
        address_family: sinan_protocol::ProbeAddressFamily::Any,
        authorization: Some(sinan_protocol::ProbeAuthorization {
            kind: sinan_protocol::ProbeAuthorizationKind::Owned,
            source: "TEST_ONLY isolated loopback listener".into(),
            scope: "This fixture process owns the exact target and method".into(),
            enabled: true,
            expires_at: None,
            identity,
        }),
    });
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
            monitor: None,
            execution_authorized: None,
            enabled: true,
        };
        authorize_fixture(&mut spec);
        let result = sample(&spec, &crate::system::SystemOps, 0).await;
        assert_eq!(result.loss_percent, 0.0);
        assert!(result.latency_ms.is_some());
        // Reserve a distinct non-listening port so parallel tests cannot reuse it.
        let closed = tokio::net::TcpSocket::new_v4()?;
        closed.bind("127.0.0.1:0".parse()?)?;
        spec.port = Some(closed.local_addr()?.port());
        authorize_fixture(&mut spec);
        let failed = sample(&spec, &crate::system::SystemOps, 0).await;
        assert_eq!(failed.loss_percent, 100.0);
        assert_eq!(failed.latency_ms, None);
        assert_eq!(failed.attempts, Some(4));
        assert_eq!(
            failed.address_family,
            Some(sinan_protocol::ProbeAddressFamily::Ipv4)
        );
        assert!(
            failed
                .error
                .as_deref()
                .is_some_and(|reason| reason.contains("TCP connection failed")
                    || reason.contains("TCP connection timed out after 1 second"))
        );
        Ok(())
    }
    #[tokio::test]
    async fn invalid_target_is_unavailable_instead_of_a_loss_measurement() {
        let mut spec = ProbeSpec {
            id: Uuid::new_v4(),
            name: "fixture".into(),
            kind: ProbeKind::Icmp,
            target: "0.0.0.0".into(),
            port: None,
            interval_secs: 10,
            carrier: String::new(),
            monitor: None,
            execution_authorized: None,
            enabled: true,
        };
        authorize_fixture(&mut spec);
        let result = sample(&spec, &crate::system::SystemOps, 0).await;
        assert!(result.error.is_some());
        assert_eq!(result.latency_ms, None);
    }
    #[tokio::test]
    async fn selected_ipv6_records_the_actual_family_and_rejects_wrong_family() -> Result<()> {
        let listener = tokio::net::TcpListener::bind("[::1]:0").await?;
        let mut spec: ProbeSpec = serde_json::from_value(
            serde_json::json!({"id":Uuid::new_v4(),"name":"TEST_ONLY owned IPv6 listener","kind":"tcp","target":"::1","port":listener.local_addr()?.port(),"interval_secs":10,"carrier":"","enabled":true}),
        )?;
        authorize_fixture(&mut spec);
        spec.monitor.as_mut().unwrap().address_family = sinan_protocol::ProbeAddressFamily::Ipv6;
        let identity = spec.identity();
        spec.monitor
            .as_mut()
            .unwrap()
            .authorization
            .as_mut()
            .unwrap()
            .identity = identity;
        let measured = sample(&spec, &crate::system::SystemOps, 0).await;
        assert_eq!(
            measured.address_family,
            Some(sinan_protocol::ProbeAddressFamily::Ipv6)
        );
        assert_eq!(measured.attempts, Some(4));
        assert_eq!(measured.loss_percent, 0.0);
        assert!(measured.latency_ms.is_some());
        assert_eq!(measured.error, None);
        spec.monitor.as_mut().unwrap().address_family = sinan_protocol::ProbeAddressFamily::Ipv4;
        let identity = spec.identity();
        spec.monitor
            .as_mut()
            .unwrap()
            .authorization
            .as_mut()
            .unwrap()
            .identity = identity;
        let denied = sample(&spec, &crate::system::SystemOps, 0).await;
        assert_eq!(denied.address_family, None);
        assert_eq!(denied.attempts, None);
        assert_eq!(denied.latency_ms, None);
        assert!(
            denied
                .error
                .as_deref()
                .is_some_and(|error| error.contains("no authorized-family unicast address"))
        );
        Ok(())
    }
    #[tokio::test]
    async fn ipv6_only_permission_never_connects_to_ipv4_listener() -> Result<()> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let mut spec = ProbeSpec {
            id: Uuid::new_v4(),
            name: "family-bound fixture".into(),
            kind: ProbeKind::Tcp,
            target: "127.0.0.1".into(),
            port: Some(listener.local_addr()?.port()),
            interval_secs: 10,
            carrier: String::new(),
            enabled: true,
            monitor: None,
            execution_authorized: None,
        };
        authorize_fixture(&mut spec);
        spec.monitor.as_mut().unwrap().address_family = sinan_protocol::ProbeAddressFamily::Ipv6;
        let identity = spec.identity();
        spec.monitor
            .as_mut()
            .unwrap()
            .authorization
            .as_mut()
            .unwrap()
            .identity = identity;
        let result = sample(&spec, &crate::system::SystemOps, 0).await;
        assert!(result.error.is_some());
        assert_eq!(result.latency_ms, None);
        assert_eq!(result.address_family, None);
        assert!(
            timeout(Duration::from_millis(50), listener.accept())
                .await
                .is_err()
        );
        Ok(())
    }

    #[tokio::test]
    async fn trusted_panel_clock_offset_enforces_authorization_expiry_before_any_connection()
    -> Result<()> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let mut spec = ProbeSpec {
            id: Uuid::new_v4(),
            name: "clock-bound fixture".into(),
            kind: ProbeKind::Tcp,
            target: "127.0.0.1".into(),
            port: Some(listener.local_addr()?.port()),
            interval_secs: 10,
            carrier: String::new(),
            enabled: true,
            monitor: None,
            execution_authorized: None,
        };
        authorize_fixture(&mut spec);
        spec.monitor
            .as_mut()
            .unwrap()
            .authorization
            .as_mut()
            .unwrap()
            .expires_at = Some(now_timestamp() + 30);
        let result = sample(&spec, &crate::system::SystemOps, 60_000).await;
        assert!(result.error.is_some());
        assert_eq!(result.latency_ms, None);
        assert!(
            timeout(Duration::from_millis(50), listener.accept())
                .await
                .is_err()
        );
        Ok(())
    }
}
