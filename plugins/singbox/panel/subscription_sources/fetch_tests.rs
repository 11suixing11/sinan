use super::*;
use std::{collections::VecDeque, io::Write, sync::Mutex};

enum PlannedResponse {
    Response {
        status: u16,
        headers: HeaderMap,
        chunks: Vec<Vec<u8>>,
    },
    Failure(&'static str),
    Pending,
    PendingBody,
}
struct SentRequest {
    url: String,
    addresses: Vec<SocketAddr>,
    headers: HeaderMap,
}
struct MockNetwork {
    resolutions: Mutex<VecDeque<Vec<SocketAddr>>>,
    responses: Mutex<VecDeque<PlannedResponse>>,
    sent: Mutex<Vec<SentRequest>>,
    dns_pending: bool,
}
impl MockNetwork {
    fn new(resolutions: Vec<Vec<SocketAddr>>, responses: Vec<PlannedResponse>) -> Self {
        Self {
            resolutions: Mutex::new(resolutions.into()),
            responses: Mutex::new(responses.into()),
            sent: Mutex::new(vec![]),
            dns_pending: false,
        }
    }
}
impl FetchNetwork for MockNetwork {
    async fn resolve(&self, _host: &str, _port: u16) -> Result<Vec<SocketAddr>, SourceFailure> {
        if self.dns_pending {
            return std::future::pending().await;
        }
        Ok(self
            .resolutions
            .lock()
            .unwrap()
            .pop_front()
            .expect("planned resolution"))
    }
    async fn get(
        &self,
        url: &Url,
        addresses: &[SocketAddr],
        headers: HeaderMap,
        _deadline: Instant,
    ) -> Result<DownloadResponse, SourceFailure> {
        self.sent.lock().unwrap().push(SentRequest {
            url: url.as_str().to_owned(),
            addresses: addresses.to_vec(),
            headers,
        });
        let planned = self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("planned response");
        match planned {
            PlannedResponse::Response {
                status,
                headers,
                chunks,
            } => Ok(DownloadResponse {
                status,
                headers,
                body: Box::pin(futures_util::stream::iter(chunks.into_iter().map(Ok))),
            }),
            PlannedResponse::Failure(kind) => Err(failure(kind, "固定失败分类")),
            PlannedResponse::Pending => std::future::pending().await,
            PlannedResponse::PendingBody => Ok(DownloadResponse {
                status: 200,
                headers: HeaderMap::new(),
                body: Box::pin(futures_util::stream::pending()),
            }),
        }
    }
}
fn config() -> FetchConfig {
    FetchConfig {
        url: "https://download.example.invalid/subscription?token=private-marker".into(),
        auth_headers: BTreeMap::new(),
        etag: None,
        last_modified: None,
    }
}
fn addresses() -> Vec<SocketAddr> {
    vec![
        "8.8.8.8:443".parse().unwrap(),
        "[2606:4700::1111]:443".parse().unwrap(),
    ]
}
fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
    pairs
        .iter()
        .map(|(k, v)| {
            (
                HeaderName::from_bytes(k.as_bytes()).unwrap(),
                HeaderValue::from_str(v).unwrap(),
            )
        })
        .collect()
}
fn response(status: u16, pairs: &[(&str, &str)], chunks: Vec<Vec<u8>>) -> PlannedResponse {
    PlannedResponse::Response {
        status,
        headers: headers(pairs),
        chunks,
    }
}
fn success<T>(result: Result<T, SourceFailure>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("unexpected {}:{}", error.stage, error.kind),
    }
}
fn error<T>(result: Result<T, SourceFailure>) -> SourceFailure {
    match result {
        Err(error) => error,
        Ok(_) => panic!("expected failure"),
    }
}
async fn run(config: &FetchConfig, network: &MockNetwork) -> Result<FetchOutcome, SourceFailure> {
    fetch_with_deadline(config, network, Instant::now() + Duration::from_secs(2)).await
}

#[test]
fn literal_targets_reject_special_ipv4_and_embedded_or_reserved_ipv6() {
    for ip in [
        "0.0.0.1",
        "10.0.0.1",
        "127.0.0.1",
        "169.254.169.254",
        "172.16.0.1",
        "192.168.0.1",
        "100.64.0.1",
        "192.0.0.9",
        "192.0.2.1",
        "192.88.99.1",
        "198.18.0.1",
        "198.51.100.1",
        "203.0.113.1",
        "224.0.0.1",
        "240.0.0.1",
        "255.255.255.255",
        "::1",
        "::ffff:8.8.8.8",
        "64:ff9b::808:808",
        "64:ff9b:1::1",
        "fc00::1",
        "fe80::1",
        "2001::1",
        "2001:20::1",
        "2001:db8::1",
        "2002:808:808::1",
        "3fff::1",
        "5f00::1",
    ] {
        assert!(!public_address(ip.parse().unwrap()), "{ip}");
    }
    for ip in [
        "8.8.8.8",
        "1.1.1.1",
        "2606:4700::1111",
        "2001:4860:4860::8888",
    ] {
        assert!(public_address(ip.parse().unwrap()), "{ip}");
    }
}

#[test]
fn url_and_auth_validation_are_write_only_and_do_not_allow_header_overrides() {
    for url in [
        "http://download.example.invalid/a",
        "file:///tmp/a",
        "https://user:pass@download.example.invalid/a",
        "https://@download.example.invalid/",
        "https://download.example.invalid/#token",
        "https://127.1/a",
        "https://2130706433/",
        "https://[::ffff:8.8.8.8]/",
        "https://localhost./",
        "https://service.local/",
        "https://download.example.invalid:0/",
        " https://download.example.invalid/",
    ] {
        assert!(validate_url(url).is_err(), "{url}");
    }
    assert!(validate_url("https://download.example.invalid:8443/sub?token=example").is_ok());
    assert!(validate_url("https://[2606:4700::1111]/sub").is_ok());
    assert!(
        validate_url(&format!(
            "https://download.example.invalid/{}",
            "中".repeat(1000)
        ))
        .is_err()
    );
    for name in [
        "User-Agent",
        "Host",
        "Origin",
        "Referer",
        "Proxy-Authorization",
        "X-Other",
    ] {
        assert!(validate_auth_headers(&BTreeMap::from([(name.into(), "example".into())])).is_err());
    }
    for value in ["", " ", "secret\r\nHost: example.invalid", "secret\t"] {
        assert!(
            validate_auth_headers(&BTreeMap::from([("authorization".into(), value.into())]))
                .is_err()
        );
    }
    let input = BTreeMap::from([
        ("Authorization".into(), "Bearer example".into()),
        ("COOKIE".into(), "account=example".into()),
        ("X-API-Key".into(), "example".into()),
    ]);
    let actual = success(validate_auth_headers(&input));
    assert_eq!(actual.len(), 3);
    assert_eq!(actual["authorization"], "Bearer example");
    assert!(
        validate_auth_headers(&BTreeMap::from([
            ("Authorization".into(), "a".into()),
            ("authorization".into(), "b".into())
        ]))
        .is_err()
    );
    assert!(
        validate_auth_headers(&BTreeMap::from([(
            "cookie".into(),
            "x".repeat(MAX_AUTH_BYTES)
        )]))
        .is_err()
    );
}

#[tokio::test]
async fn mixed_dns_answers_are_rejected_before_any_connection() {
    for blocked in ["127.0.0.1:443", "10.0.0.1:443", "[::ffff:8.8.8.8]:443"] {
        let network =
            MockNetwork::new(vec![vec![addresses()[0], blocked.parse().unwrap()]], vec![]);
        assert_eq!(
            error(run(&config(), &network).await).kind,
            "private_address"
        );
        assert!(network.sent.lock().unwrap().is_empty());
    }
    for answers in [
        vec![],
        vec![addresses()[0]; MAX_DNS_ADDRESSES + 1],
        vec!["8.8.8.8:8443".parse().unwrap()],
    ] {
        let network = MockNetwork::new(vec![answers], vec![]);
        assert!(run(&config(), &network).await.is_err());
        assert!(network.sent.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn checked_addresses_and_secrets_are_only_sent_to_the_same_origin() {
    let mut config = config();
    config
        .auth_headers
        .insert("Authorization".into(), "Bearer example-secret".into());
    config.etag = Some("\"v1\"".into());
    let first = addresses();
    let second = vec!["1.1.1.1:443".parse().unwrap()];
    let network = MockNetwork::new(
        vec![first.clone(), second.clone()],
        vec![
            response(302, &[("location", "/next?token=second")], vec![]),
            response(
                200,
                &[("etag", "\"v2\"")],
                vec![b"trojan://example@proxy.example.invalid:443".to_vec()],
            ),
        ],
    );
    match success(run(&config, &network).await) {
        FetchOutcome::Modified { body, etag, .. } => {
            assert!(body.starts_with(b"trojan://"));
            assert_eq!(etag.as_deref(), Some("\"v2\""));
        }
        _ => panic!("expected modified"),
    }
    let sent = network.sent.lock().unwrap();
    assert_eq!(sent.len(), 2);
    assert_eq!(
        sent[0].addresses.iter().copied().collect::<BTreeSet<_>>(),
        first.into_iter().collect()
    );
    assert_eq!(sent[1].addresses, second);
    assert!(sent[1].url.ends_with("/next?token=second"));
    for request in sent.iter() {
        assert_eq!(request.headers["authorization"], "Bearer example-secret");
        assert!(request.headers["authorization"].is_sensitive());
        assert!(request.headers.get("referer").is_none());
        assert!(request.headers.get("user-agent").is_none());
        assert_eq!(request.headers["if-none-match"], "\"v1\"");
        assert!(request.headers["if-none-match"].is_sensitive());
    }
}

#[tokio::test]
async fn cross_origin_and_excess_redirects_never_send_the_next_request() {
    for target in [
        "https://other.example.invalid/a",
        "http://download.example.invalid/a",
        "https://download.example.invalid:8443/a",
        "https://127.0.0.1/a",
        "https://user:pass@download.example.invalid/a",
        "https://@download.example.invalid/a",
        "//@download.example.invalid/a",
        "https:////@download.example.invalid/a",
        "https://download.example.invalid/\tignored",
    ] {
        let network = MockNetwork::new(
            vec![addresses()],
            vec![response(302, &[("location", target)], vec![])],
        );
        let failure = error(run(&config(), &network).await);
        assert!(matches!(
            failure.kind.as_str(),
            "redirect_origin" | "redirect" | "url" | "private_address"
        ));
        assert_eq!(network.sent.lock().unwrap().len(), 1);
        assert!(!serde_json::to_string(&failure).unwrap().contains(target));
    }
    let network = MockNetwork::new(
        vec![addresses(); 4],
        (0..4)
            .map(|_| response(307, &[("location", "/next")], vec![]))
            .collect(),
    );
    assert_eq!(error(run(&config(), &network).await).kind, "redirect_limit");
    assert_eq!(network.sent.lock().unwrap().len(), 4);
}

#[tokio::test]
async fn redirects_recheck_dns_and_block_rebinding() {
    let network = MockNetwork::new(
        vec![addresses(), vec!["169.254.169.254:443".parse().unwrap()]],
        vec![response(302, &[("location", "/next")], vec![])],
    );
    assert_eq!(
        error(run(&config(), &network).await).kind,
        "private_address"
    );
    assert_eq!(network.sent.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn http_failures_have_distinct_classes_without_response_or_url_content() {
    for (status, kind) in [(403, "http_403"), (429, "http_429"), (500, "http")] {
        let network = MockNetwork::new(
            vec![addresses()],
            vec![response(
                status,
                &[],
                vec![b"private-marker password secret".to_vec()],
            )],
        );
        let failure = error(run(&config(), &network).await);
        assert_eq!(failure.kind, kind);
        assert_eq!(failure.http_status, Some(status));
        let output = serde_json::to_string(&failure).unwrap();
        for secret in [
            "private-marker",
            "password",
            "download.example.invalid",
            "subscription?",
            "secret",
        ] {
            assert!(!output.contains(secret));
        }
    }
    let network = MockNetwork::new(
        vec![addresses()],
        vec![response(
            200,
            &[("content-type", "text/html; charset=utf-8")],
            vec![b"<html>private-marker</html>".to_vec()],
        )],
    );
    assert_eq!(
        error(run(&config(), &network).await).kind,
        "non_subscription"
    );
}

#[tokio::test]
async fn conditional_304_requires_existing_cache_and_returns_no_new_body() {
    let network = MockNetwork::new(vec![addresses()], vec![response(304, &[], vec![])]);
    assert_eq!(error(run(&config(), &network).await).kind, "cache");
    let mut config = config();
    config.etag = Some("\"old\"".into());
    let network = MockNetwork::new(
        vec![addresses()],
        vec![response(304, &[("etag", "\"new\"")], vec![])],
    );
    assert!(
        matches!(success(run(&config, &network).await), FetchOutcome::NotModified { etag: Some(ref v), .. } if v == "\"new\"")
    );
}

#[tokio::test]
async fn total_deadline_covers_dns_and_body_waiting() {
    let mut dns = MockNetwork::new(vec![], vec![]);
    dns.dns_pending = true;
    assert_eq!(
        error(
            fetch_with_deadline(&config(), &dns, Instant::now() + Duration::from_millis(10)).await
        )
        .kind,
        "timeout"
    );
    let network = MockNetwork::new(vec![addresses()], vec![PlannedResponse::Pending]);
    assert_eq!(
        error(
            fetch_with_deadline(
                &config(),
                &network,
                Instant::now() + Duration::from_millis(10)
            )
            .await
        )
        .kind,
        "timeout"
    );
    let body = MockNetwork::new(vec![addresses()], vec![PlannedResponse::PendingBody]);
    assert_eq!(
        error(
            fetch_with_deadline(&config(), &body, Instant::now() + Duration::from_millis(10)).await
        )
        .kind,
        "timeout"
    );
}

#[tokio::test]
async fn transport_failures_keep_typed_errors_and_do_not_retry() {
    for kind in ["connection", "tls", "timeout"] {
        let network = MockNetwork::new(vec![addresses()], vec![PlannedResponse::Failure(kind)]);
        assert_eq!(error(run(&config(), &network).await).kind, kind);
        assert_eq!(network.sent.lock().unwrap().len(), 1);
    }
}

fn gzip(input: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(input).unwrap();
    encoder.finish().unwrap()
}

#[tokio::test]
async fn wire_length_and_accumulated_chunks_are_bounded() {
    let network = MockNetwork::new(
        vec![addresses()],
        vec![response(
            200,
            &[("content-length", &(MAX_CONTENT_BYTES + 1).to_string())],
            vec![],
        )],
    );
    assert_eq!(error(run(&config(), &network).await).kind, "body_limit");
    let network = MockNetwork::new(
        vec![addresses()],
        vec![response(
            200,
            &[],
            vec![vec![b'a'; MAX_CONTENT_BYTES], vec![b'b']],
        )],
    );
    assert_eq!(error(run(&config(), &network).await).kind, "body_limit");
    let network = MockNetwork::new(
        vec![addresses()],
        vec![response(200, &[], vec![vec![b'a'; MAX_CONTENT_BYTES]])],
    );
    assert!(
        matches!(success(run(&config(), &network).await), FetchOutcome::Modified { ref body, .. } if body.len() == MAX_CONTENT_BYTES)
    );
}

#[test]
fn decompression_checks_output_members_encoding_and_deadline() {
    let deadline = Instant::now() + Duration::from_secs(2);
    let input = b"trojan://example@proxy.example.invalid:443";
    assert_eq!(
        success(decode_body(gzip(input), Some("gzip"), deadline)),
        input
    );
    let mut members = gzip(b"first");
    members.extend(gzip(b"second"));
    assert_eq!(
        success(decode_body(members, Some("gzip"), deadline)),
        b"firstsecond"
    );
    assert_eq!(
        error(decode_body(
            gzip(&vec![b'a'; MAX_CONTENT_BYTES + 1]),
            Some("gzip"),
            deadline
        ))
        .kind,
        "decompressed_limit"
    );
    assert_eq!(
        error(decode_body(b"not-gzip".to_vec(), Some("gzip"), deadline)).kind,
        "encoding"
    );
    assert_eq!(
        error(decode_body(gzip(input), Some("br"), deadline)).kind,
        "encoding"
    );
    assert_eq!(
        error(decode_body(
            vec![],
            None,
            Instant::now() - Duration::from_secs(1)
        ))
        .kind,
        "timeout"
    );
    let mut truncated = gzip(input);
    truncated.truncate(truncated.len() - 4);
    assert_eq!(
        error(decode_body(truncated, Some("gzip"), deadline)).kind,
        "encoding"
    );
}

#[tokio::test]
async fn real_tls_transport_failure_is_classified_without_exposing_the_url() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut hello = [0u8; 2048];
        let _ = stream.read(&mut hello).await;
        let _ = stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
            .await;
    });
    // Exercise only the private TLS transport. The public fetch path rejects
    // this loopback target before connecting, as independently asserted above.
    let url = Url::parse(&format!(
        "https://tls.example.invalid:{}/private-marker",
        address.port()
    ))
    .unwrap();
    let failure = error(
        PublicNetwork
            .get(
                &url,
                &[address],
                HeaderMap::new(),
                Instant::now() + Duration::from_secs(2),
            )
            .await,
    );
    server.await.unwrap();
    assert_eq!(failure.kind, "tls");
    assert!(
        !serde_json::to_string(&failure)
            .unwrap()
            .contains("private-marker")
    );
}
