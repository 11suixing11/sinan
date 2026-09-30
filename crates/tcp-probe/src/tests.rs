use crate::{
    engine::{Limits, Network, NetworkFuture, run_with},
    *,
};
use sha2::{Digest, Sha256};
use std::{
    net::SocketAddr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

mod fixture;
mod safety;
use fixture::{Directory, target};

#[tokio::test]
async fn real_ipv4_and_ipv6_connections_close_without_application_data() {
    use tokio::{io::AsyncReadExt, net::TcpListener};
    for host in ["127.0.0.1", "::1"] {
        let listener = TcpListener::bind((host, 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            for _ in 0..4 {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut byte = [0; 1];
                assert_eq!(
                    tokio::time::timeout(Duration::from_secs(2), stream.read(&mut byte))
                        .await
                        .unwrap()
                        .unwrap(),
                    0
                );
            }
        });
        let directory = Directory::new();
        let (options, mut journal) = directory
            .prepare(
                vec![target(1, host, address.port())],
                if host.contains(':') {
                    IpVersion::V6
                } else {
                    IpVersion::V4
                },
            )
            .await;
        let report = run(&options, &mut journal).await.unwrap();
        assert!(report.complete);
        let result = &report.targets[0];
        assert_eq!(result.dns_attempts, 0);
        assert_eq!(result.address, Some(address.to_string()));
        assert_eq!(result.samples.len(), 4);
        assert!(
            result
                .samples
                .iter()
                .all(|sample| sample.error.is_none() && sample.latency_ms.is_some())
        );
        assert_eq!(result.summary.connection_success_percent, Some(100.0));
        server.await.unwrap();
        directory.assert_reports(&report);
    }
}

#[tokio::test]
async fn refusal_and_missing_family_do_not_fabricate_latency() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let directory = Directory::new();
    let (options, mut journal) = directory
        .prepare(
            vec![target(1, "127.0.0.1", port), target(2, "::1", port)],
            IpVersion::V4,
        )
        .await;
    let report = run(&options, &mut journal).await.unwrap();
    assert!(report.complete);
    assert_eq!(
        report.targets[0].summary.connection_success_percent,
        Some(0.0)
    );
    assert_eq!(report.targets[0].summary.latency_mean_ms, None);
    assert!(
        report.targets[0]
            .samples
            .iter()
            .all(|sample| sample.latency_ms.is_none()
                && sample.error.as_deref() == Some("connect_refused"))
    );
    assert_eq!(report.targets[1].summary.connection_success_percent, None);
    assert_eq!(report.targets[1].summary.latency_mean_ms, None);
    assert_eq!(
        report.targets[1].error.as_deref(),
        Some("ip_family_unavailable")
    );
    directory.assert_reports(&report);
}

enum Dns {
    Addresses(Vec<SocketAddr>),
    Error,
    Pending,
}
struct FakeNetwork {
    dns: Dns,
    dns_calls: AtomicUsize,
    active: Arc<AtomicUsize>,
    peak: AtomicUsize,
    connections: Mutex<Vec<SocketAddr>>,
    pending: bool,
}
impl FakeNetwork {
    fn new(dns: Dns, pending: bool) -> Arc<Self> {
        Arc::new(Self {
            dns,
            dns_calls: AtomicUsize::new(0),
            active: Arc::new(AtomicUsize::new(0)),
            peak: AtomicUsize::new(0),
            connections: Mutex::new(Vec::new()),
            pending,
        })
    }
}
struct Active(Arc<AtomicUsize>);
impl Drop for Active {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}
impl Network for FakeNetwork {
    fn resolve<'a>(&'a self, _host: &'a str, _port: u16) -> NetworkFuture<'a, Vec<SocketAddr>> {
        self.dns_calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            match &self.dns {
                Dns::Addresses(addresses) => Ok(addresses.clone()),
                Dns::Error => Err(std::io::Error::other("fixture DNS failure")),
                Dns::Pending => std::future::pending().await,
            }
        })
    }
    fn connect(&self, address: SocketAddr) -> NetworkFuture<'_, ()> {
        self.connections.lock().unwrap().push(address);
        Box::pin(async move {
            let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
            self.peak.fetch_max(active, Ordering::SeqCst);
            let _guard = Active(self.active.clone());
            if self.pending {
                std::future::pending::<()>().await;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
            Ok(())
        })
    }
}
fn fast_limits() -> Limits {
    Limits {
        dns: Duration::from_millis(20),
        connect: Duration::from_millis(60),
        interval: Duration::from_millis(1),
        total: Duration::from_secs(2),
        publication: Duration::from_millis(250),
    }
}

#[tokio::test]
async fn verified_snapshot_is_frozen_and_public_options_cannot_replace_its_digest() {
    let directory = Directory::new();
    let (options, mut journal) = directory
        .prepare(vec![target(1, "127.0.0.1", 12345)], IpVersion::V4)
        .await;
    std::fs::write(
        directory.path.join("targets.json"),
        serde_json::to_vec(&Snapshot {
            schema: 1,
            targets: vec![target(2, "127.0.0.2", 54321)],
        })
        .unwrap(),
    )
    .unwrap();
    let network = FakeNetwork::new(Dns::Error, false);
    let report = run_with(&options, &mut journal, network.clone(), fast_limits())
        .await
        .unwrap();
    assert_eq!(report.target_digest, options.target_digest);
    assert_eq!(
        report.targets[0].target.id,
        target(1, "127.0.0.1", 12345).id
    );
    assert!(
        network
            .connections
            .lock()
            .unwrap()
            .iter()
            .all(|address| *address == "127.0.0.1:12345".parse::<SocketAddr>().unwrap())
    );
    let mut changed = options;
    changed.target_digest = "0".repeat(64);
    assert!(
        run_with(&changed, &mut journal, network, fast_limits())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn dns_runs_once_and_one_matching_socket_is_used_with_bounded_parallelism() {
    let addresses = vec![
        "[::1]:12345".parse().unwrap(),
        "127.0.0.1:12345".parse().unwrap(),
        "127.0.0.2:12345".parse().unwrap(),
    ];
    for concurrency in [1, 2] {
        let network = FakeNetwork::new(Dns::Addresses(addresses.clone()), false);
        let directory = Directory::new();
        let (mut options, mut journal) = directory
            .prepare(
                vec![
                    target(1, "first.example.test", 12345),
                    target(2, "second.example.test", 12345),
                ],
                IpVersion::V4,
            )
            .await;
        options.count = 8;
        options.concurrency = concurrency;
        let report = run_with(&options, &mut journal, network.clone(), fast_limits())
            .await
            .unwrap();
        assert!(report.complete);
        assert_eq!(network.dns_calls.load(Ordering::SeqCst), 2);
        assert!(
            report
                .targets
                .iter()
                .all(|target| target.dns_attempts == 1 && target.samples.len() == 8)
        );
        assert_eq!(network.connections.lock().unwrap().len(), 16);
        assert!(
            network
                .connections
                .lock()
                .unwrap()
                .iter()
                .all(|address| *address == addresses[1])
        );
        assert!(network.peak.load(Ordering::SeqCst) <= usize::from(concurrency));
        assert_eq!(network.active.load(Ordering::SeqCst), 0);
        directory.assert_reports(&report);
    }
}

#[tokio::test]
async fn dns_failure_timeout_and_no_matching_family_preserve_unknown_results() {
    for (dns, error) in [
        (Dns::Error, "dns_error"),
        (Dns::Pending, "dns_timeout"),
        (
            Dns::Addresses(vec!["[::1]:12345".parse().unwrap()]),
            "ip_family_unavailable",
        ),
    ] {
        let directory = Directory::new();
        let network = FakeNetwork::new(dns, false);
        let (options, mut journal) = directory
            .prepare(
                vec![target(1, "unknown.example.test", 12345)],
                IpVersion::V4,
            )
            .await;
        let report = run_with(&options, &mut journal, network.clone(), fast_limits())
            .await
            .unwrap();
        assert!(report.complete);
        assert_eq!(report.targets[0].error.as_deref(), Some(error));
        assert_eq!(report.targets[0].summary.connection_success_percent, None);
        assert_eq!(report.targets[0].summary.latency_mean_ms, None);
        assert!(network.connections.lock().unwrap().is_empty());
        assert_eq!(network.dns_calls.load(Ordering::SeqCst), 1);
        directory.assert_reports(&report);
    }
}

#[tokio::test]
async fn deadline_includes_queued_targets_and_preserves_atomic_partial_sections() {
    let directory = Directory::new();
    let network = FakeNetwork::new(Dns::Error, true);
    let (options, mut journal) = directory
        .prepare(
            (1..=8).map(|id| target(id, "127.0.0.1", 12345)).collect(),
            IpVersion::V4,
        )
        .await;
    let mut limits = fast_limits();
    limits.total = Duration::from_secs(1);
    limits.publication = Duration::from_millis(300);
    limits.connect = Duration::from_millis(250);
    let started = std::time::Instant::now();
    let report = run_with(&options, &mut journal, network.clone(), limits)
        .await
        .unwrap();
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(!report.complete && report.deadline_exceeded);
    assert_eq!(network.active.load(Ordering::SeqCst), 0);
    assert!(network.connections.lock().unwrap().len() <= 4);
    assert!(
        report.targets[0]
            .samples
            .iter()
            .all(|sample| sample.error.as_deref() == Some("connect_timeout")
                && sample.latency_ms.is_none())
    );
    assert!(
        report.targets[1..]
            .iter()
            .all(|target| target.status == "not_attempted"
                && target.samples.is_empty()
                && target.summary.connection_success_percent.is_none())
    );
    directory.assert_reports(&report);
}

#[tokio::test]
async fn outer_cancellation_drops_probes_and_keeps_previously_published_sections() {
    let directory = Directory::new();
    let network = FakeNetwork::new(Dns::Error, true);
    let (options, mut journal) = directory
        .prepare(vec![target(1, "127.0.0.1", 12345)], IpVersion::V4)
        .await;
    let handle = tokio::spawn({
        let network = network.clone();
        async move { run_with(&options, &mut journal, network, fast_limits()).await }
    });
    tokio::time::timeout(Duration::from_secs(1), async {
        while network.active.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    handle.abort();
    assert!(handle.await.unwrap_err().is_cancelled());
    assert_eq!(network.active.load(Ordering::SeqCst), 0);
    let body: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.path.join("result.json")).unwrap())
            .unwrap();
    assert_eq!(body["complete"], false);
    assert_eq!(
        body["targets"][0]["summary"]["latency_mean_ms"],
        serde_json::Value::Null
    );
    assert!(directory.path.join("sections/tcp_scope.json").is_file());
}
