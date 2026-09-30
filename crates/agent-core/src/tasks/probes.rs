use crate::{SharedState, artifacts::PanelClient};
use anyhow::{Result, ensure};
use sinan_adapter_sdk::Privileged;
use sinan_protocol::{
    ProbeBatch, ProbeKind, ProbeResult, ProbeSpec, TaskAck, now_timestamp, telemetry::now_millis,
};
use std::{collections::HashMap, path::Path, sync::Arc, time::Duration};
use tokio::{
    net::{TcpStream, lookup_host},
    sync::{Semaphore, watch},
    task::JoinSet,
    time::{Instant, timeout},
};
use uuid::Uuid;

pub(super) async fn run(
    state: SharedState,
    ops: Arc<dyn Privileged>,
    clients: watch::Receiver<Option<Arc<PanelClient>>>,
) -> Result<()> {
    tokio::try_join!(sample_loop(state.clone(), ops), synchronize(state, clients))?;
    Ok(())
}

async fn synchronize(
    state: SharedState,
    clients: watch::Receiver<Option<Arc<PanelClient>>>,
) -> Result<()> {
    let mut refreshed = Instant::now() - Duration::from_secs(31);
    loop {
        let client = clients.borrow().clone();
        if let Some(client) = client {
            if refreshed.elapsed() >= Duration::from_secs(30) {
                refreshed = Instant::now();
                match client
                    .get_json::<Vec<ProbeSpec>>("/api/agent/v1/probes")
                    .await
                {
                    Ok(specs) if specs.len() <= 32 && specs.iter().all(ProbeSpec::valid) => {
                        state
                            .lock()
                            .map_err(|_| anyhow::anyhow!("state lock poisoned"))?
                            .set_json("probes:configuration", &(now_timestamp(), specs))?;
                    }
                    Ok(_) => tracing::warn!("panel provided invalid probes"),
                    Err(error) => tracing::warn!(%error,"probe configuration refresh failed"),
                }
            }
            if let Err(error) = upload(&state, &client).await {
                tracing::warn!(%error,"probe results retained for retry");
            }
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

async fn sample_loop(state: SharedState, ops: Arc<dyn Privileged>) -> Result<()> {
    let mut due = HashMap::<Uuid, Instant>::new();
    let permits = Arc::new(Semaphore::new(4));
    loop {
        let configuration = state
            .lock()
            .map_err(|_| anyhow::anyhow!("state lock poisoned"))?
            .get_json::<(i64, Vec<ProbeSpec>)>("probes:configuration")?;
        if let Some((fetched, specs)) = configuration
            && fetched > now_timestamp() - 86400
        {
            due.retain(|id, _| specs.iter().any(|s| s.id == *id && s.enabled));
            let mut tasks = JoinSet::new();
            for spec in specs.into_iter().filter(|s| s.enabled) {
                if due.get(&spec.id).is_some_and(|next| *next > Instant::now()) {
                    continue;
                }
                due.insert(
                    spec.id,
                    Instant::now() + Duration::from_secs(u64::from(spec.interval_secs)),
                );
                let ops = ops.clone();
                let permits = permits.clone();
                tasks.spawn(async move {
                    let _permit = permits.acquire_owned().await?;
                    Ok::<_, anyhow::Error>(sample(&spec, ops.as_ref()).await)
                });
            }
            while let Some(result) = tasks.join_next().await {
                let mut result = result??;
                let mut state = state
                    .lock()
                    .map_err(|_| anyhow::anyhow!("state lock poisoned"))?;
                result.sampled_at += state.get_json::<i64>("clock_offset_ms")?.unwrap_or(0);
                state.save_probe_result(&result)?;
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
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
    match measure(spec, ops).await {
        Ok(times) => {
            result.loss_percent = (4 - times.len().min(4)) as f64 * 25.0;
            if !times.is_empty() {
                result.latency_ms = Some(times.iter().sum::<f64>() / times.len() as f64);
            }
        }
        Err(error) => result.error = Some(error.to_string().chars().take(512).collect()),
    }
    result
}

async fn measure(spec: &ProbeSpec, ops: &dyn Privileged) -> Result<Vec<f64>> {
    ensure!(spec.valid(), "invalid probe configuration");
    let addresses = timeout(
        Duration::from_secs(2),
        lookup_host((spec.target.as_str(), spec.port.unwrap_or(0))),
    )
    .await??;
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
            Ok(times)
        }
        ProbeKind::Icmp => {
            #[cfg(target_os = "windows")]
            let (program, mut args) = (
                Path::new("ping.exe"),
                vec!["-n".into(), "4".into(), "-w".into(), "1000".into()],
            );
            #[cfg(target_os = "linux")]
            let (program, mut args) = (
                Path::new("ping"),
                vec![
                    "-n".into(),
                    "-c".into(),
                    "4".into(),
                    "-W".into(),
                    "1".into(),
                ],
            );
            #[cfg(any(target_os = "macos", target_os = "freebsd"))]
            let (program, mut args) = if address.is_ipv6() {
                (
                    Path::new("ping6"),
                    vec!["-n".into(), "-c".into(), "4".into()],
                )
            } else {
                (
                    Path::new("ping"),
                    vec![
                        "-n".into(),
                        "-c".into(),
                        "4".into(),
                        "-W".into(),
                        "1000".into(),
                    ],
                )
            };
            args.push(address.ip().to_string());
            let output = ops.execute_bounded(program, &args, 8, 16 * 1024).await?;
            ensure!(!output.timed_out, "ICMP probe timed out");
            let times = parse_ping(&output.output.stdout);
            if times.is_empty() && !output.output.success && !output.output.stderr.trim().is_empty()
            {
                anyhow::bail!(
                    "ICMP probe failed: {}",
                    output
                        .output
                        .stderr
                        .trim()
                        .chars()
                        .take(256)
                        .collect::<String>()
                );
            }
            Ok(times)
        }
    }
}

fn parse_ping(text: &str) -> Vec<f64> {
    text.lines()
        .filter_map(|line| {
            let tail = ["time=", "time<", "时间=", "时间<"]
                .into_iter()
                .find_map(|marker| line.split_once(marker).map(|(_, tail)| (marker, tail)))?;
            let number: String = tail
                .1
                .chars()
                .take_while(|c| c.is_ascii_digit() || matches!(c, '.' | ','))
                .collect();
            let value = number.replace(',', ".").parse::<f64>().ok()?;
            let value = if tail.0.ends_with('<') {
                value / 2.0
            } else {
                value
            };
            (value.is_finite() && (0.0..=60_000.0).contains(&value)).then_some(value)
        })
        .take(4)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn tcp_probe_measures_a_real_listener_and_closed_port() -> Result<()> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let spec = ProbeSpec {
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
        drop(listener);
        let failed = sample(&spec, &crate::system::SystemOps).await;
        assert_eq!(failed.loss_percent, 100.0);
        assert_eq!(failed.latency_ms, None);
        Ok(())
    }
    #[test]
    fn ping_parser_handles_both_languages_and_missing_samples() {
        assert_eq!(
            parse_ping("64 bytes time=1.25 ms\nReply time<1ms\n字节=32 时间=2ms"),
            vec![1.25, 0.5, 2.0]
        );
        assert!(parse_ping("100% packet loss").is_empty());
    }
}
