use super::models::{MAX_CONTENT_BYTES, SourceFailure};
use futures_util::{Stream, StreamExt};
use reqwest::{
    Client, Url,
    header::{HeaderMap, HeaderName, HeaderValue},
    redirect::Policy,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    io::{Cursor, Read},
    net::{IpAddr, SocketAddr},
    pin::Pin,
    time::Duration,
};
use tokio::time::Instant;

const FETCH_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_URL_BYTES: usize = 8192;
const MAX_AUTH_BYTES: usize = 8192;
const MAX_CACHE_HEADER_BYTES: usize = 2048;
const MAX_DNS_ADDRESSES: usize = 64;

// These types deliberately do not implement Debug: paths and headers are secrets.
pub(crate) struct FetchConfig {
    pub url: String,
    pub auth_headers: BTreeMap<String, String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

pub(crate) enum FetchOutcome {
    Modified {
        body: Vec<u8>,
        etag: Option<String>,
        last_modified: Option<String>,
    },
    NotModified {
        etag: Option<String>,
        last_modified: Option<String>,
    },
}

fn failure(kind: &str, message: &str) -> SourceFailure {
    SourceFailure::new("fetch", kind, message)
}

pub(crate) fn validate_auth_headers(
    input: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, SourceFailure> {
    let mut result = BTreeMap::new();
    let mut bytes = 0usize;
    if input.len() > 3 {
        return Err(failure("auth_headers", "认证头最多三项"));
    }
    for (name, value) in input {
        let name = name.to_ascii_lowercase();
        if !matches!(name.as_str(), "authorization" | "cookie" | "x-api-key")
            || value.is_empty()
            || value.trim().is_empty()
            || value.bytes().any(|b| b.is_ascii_control())
            || HeaderValue::from_str(value).is_err()
        {
            return Err(failure("auth_headers", "认证头名称或内容不符合要求"));
        }
        bytes = bytes.saturating_add(name.len()).saturating_add(value.len());
        if bytes > MAX_AUTH_BYTES || result.insert(name, value.clone()).is_some() {
            return Err(failure("auth_headers", "认证头重复或超过大小限制"));
        }
    }
    Ok(result)
}

pub(crate) fn validate_url(input: &str) -> Result<Url, SourceFailure> {
    if input.is_empty()
        || input.len() > MAX_URL_BYTES
        || input.trim() != input
        || input.bytes().any(|b| b.is_ascii_control() || b == b'\\')
        || input
            .get(..8)
            .is_none_or(|prefix| !prefix.eq_ignore_ascii_case("https://"))
    {
        return Err(failure("url", "订阅地址为空、包含控制字符或超过大小限制"));
    }
    let url = Url::parse(input).map_err(|_| failure("url", "订阅地址格式不合法"))?;
    if url.as_str().len() > MAX_URL_BYTES
        || url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.host().is_none()
        || url.port_or_known_default().is_none_or(|port| port == 0)
    {
        return Err(failure("url", "订阅地址须为无用户信息和片段的 HTTPS 地址"));
    }
    let host = hostname(&url)?;
    if let Ok(ip) = host.parse::<IpAddr>() {
        if !public_address(ip) {
            return Err(failure("private_address", "订阅地址不属于允许公网范围"));
        }
    } else {
        let host = host.trim_end_matches('.');
        if host == "localhost" || host.ends_with(".localhost") || host.ends_with(".local") {
            return Err(failure("private_address", "订阅地址不属于允许公网范围"));
        }
    }
    // URL normalization can discard an empty userinfo marker; reject it too.
    if input.split_once("://").is_some_and(|(_, rest)| {
        rest.split(['/', '?', '#'])
            .next()
            .is_none_or(|a| a.is_empty() || a.contains('@'))
    }) {
        return Err(failure("url", "订阅地址不能携带用户信息"));
    }
    Ok(url)
}

fn public_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(ip) => {
            let b = ip.octets();
            !ip.is_private()
                && !ip.is_loopback()
                && !ip.is_link_local()
                && !ip.is_multicast()
                && !ip.is_broadcast()
                && !ip.is_documentation()
                && b[0] != 0
                && b[0] < 224
                && !(b[0] == 100 && (64..=127).contains(&b[1]))
                && !(b[0] == 198 && matches!(b[1], 18 | 19))
                && !(b[0] == 192 && b[1] == 0 && b[2] == 0)
                && !(b[0] == 192 && b[1] == 88 && b[2] == 99)
        }
        IpAddr::V6(ip) => {
            let s = ip.segments();
            // Restrict to ordinary global unicast, excluding protocol assignments,
            // tunnels, documentation and all IPv4 embedding/translation prefixes.
            s[0] & 0xe000 == 0x2000
                && !(s[0] == 0x2001 && s[1] < 0x0200)
                && !(s[0] == 0x2001 && s[1] == 0x0db8)
                && s[0] != 0x2002
                && !(s[0] == 0x3fff && s[1] & 0xf000 == 0)
        }
    }
}

fn checked_addresses(
    addresses: Vec<SocketAddr>,
    port: u16,
) -> Result<Vec<SocketAddr>, SourceFailure> {
    if addresses.is_empty() {
        return Err(failure("dns", "订阅主机没有可用地址"));
    }
    if addresses.len() > MAX_DNS_ADDRESSES {
        return Err(failure("dns_limit", "订阅主机返回过多地址"));
    }
    if addresses
        .iter()
        .any(|a| a.port() != port || !public_address(a.ip()))
    {
        return Err(failure("private_address", "订阅地址解析到非允许公网范围"));
    }
    Ok(addresses
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect())
}

fn hostname(url: &Url) -> Result<String, SourceFailure> {
    let host = url
        .host_str()
        .ok_or_else(|| failure("url", "订阅地址缺少主机"))?;
    // Url displays IPv6 hosts with brackets; DNS and IP parsing need the bare host.
    Ok(host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host)
        .to_owned())
}

fn request_headers(config: &FetchConfig) -> Result<HeaderMap, SourceFailure> {
    let mut headers = HeaderMap::new();
    for (name, value) in validate_auth_headers(&config.auth_headers)? {
        let name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| failure("auth_headers", "认证头名称不合法"))?;
        let mut value = HeaderValue::from_str(&value)
            .map_err(|_| failure("auth_headers", "认证头内容不合法"))?;
        value.set_sensitive(true);
        headers.insert(name, value);
    }
    for (name, value) in [
        (reqwest::header::IF_NONE_MATCH, config.etag.as_ref()),
        (
            reqwest::header::IF_MODIFIED_SINCE,
            config.last_modified.as_ref(),
        ),
    ] {
        if let Some(value) = value {
            if value.is_empty()
                || value.len() > MAX_CACHE_HEADER_BYTES
                || value.bytes().any(|b| b.is_ascii_control())
            {
                return Err(failure("cache", "条件请求缓存无效"));
            }
            let mut value =
                HeaderValue::from_str(value).map_err(|_| failure("cache", "条件请求缓存无效"))?;
            value.set_sensitive(true);
            headers.insert(name, value);
        }
    }
    headers.insert(
        reqwest::header::ACCEPT_ENCODING,
        HeaderValue::from_static("gzip, identity"),
    );
    Ok(headers)
}

type ResponseBody = Pin<Box<dyn Stream<Item = Result<Vec<u8>, SourceFailure>> + Send>>;
struct DownloadResponse {
    status: u16,
    headers: HeaderMap,
    body: ResponseBody,
}

// The production implementation never substitutes an unchecked address. Tests
// replace this private transport to inspect exactly what was checked and sent.
trait FetchNetwork: Sync {
    fn resolve(
        &self,
        host: &str,
        port: u16,
    ) -> impl Future<Output = Result<Vec<SocketAddr>, SourceFailure>> + Send;
    fn get(
        &self,
        url: &Url,
        addresses: &[SocketAddr],
        headers: HeaderMap,
        deadline: Instant,
    ) -> impl Future<Output = Result<DownloadResponse, SourceFailure>> + Send;
}

struct PublicNetwork;
impl FetchNetwork for PublicNetwork {
    async fn resolve(&self, host: &str, port: u16) -> Result<Vec<SocketAddr>, SourceFailure> {
        if let Ok(ip) = host.parse::<IpAddr>() {
            return Ok(vec![SocketAddr::new(ip, port)]);
        }
        let addresses = tokio::net::lookup_host((host, port))
            .await
            .map_err(|_| failure("dns", "订阅主机 DNS 查询失败"))?;
        Ok(addresses.take(MAX_DNS_ADDRESSES + 1).collect())
    }

    async fn get(
        &self,
        url: &Url,
        addresses: &[SocketAddr],
        headers: HeaderMap,
        deadline: Instant,
    ) -> Result<DownloadResponse, SourceFailure> {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(failure("timeout", "订阅获取超过总时间限制"));
        }
        let host = hostname(url)?;
        let client = Client::builder()
            .no_proxy()
            .https_only(true)
            .redirect(Policy::none())
            .referer(false)
            .resolve_to_addrs(&host, addresses)
            .connect_timeout(remaining)
            .timeout(remaining)
            .build()
            .map_err(|_| failure("connection", "无法初始化订阅连接"))?;
        let response = client
            .get(url.clone())
            .headers(headers)
            .send()
            .await
            .map_err(network_failure)?;
        let status = response.status().as_u16();
        let headers = response.headers().clone();
        let body = Box::pin(response.bytes_stream().map(|chunk| {
            let chunk = chunk.map_err(network_failure)?;
            if chunk.len() > MAX_CONTENT_BYTES {
                return Err(failure("body_limit", "订阅正文超过 2 MiB 限制"));
            }
            Ok(chunk.to_vec())
        }));
        Ok(DownloadResponse {
            status,
            headers,
            body,
        })
    }
}

fn network_failure(error: reqwest::Error) -> SourceFailure {
    if error.is_timeout() {
        return failure("timeout", "订阅获取超过总时间限制");
    }
    if tls_cause(&error, 0) {
        return failure("tls", "订阅 HTTPS 证书或 TLS 握手失败");
    }
    if error.is_connect() {
        failure("connection", "无法连接订阅服务")
    } else {
        failure("connection", "订阅响应读取失败")
    }
}

fn tls_cause(error: &(dyn std::error::Error + 'static), depth: usize) -> bool {
    if depth > 16 {
        return false;
    }
    if error.downcast_ref::<rustls::Error>().is_some() {
        return true;
    }
    // io::Error::source forwards the wrapped error's source. Inspect get_ref
    // as well so the actual rustls error is not skipped during TLS failures.
    if error
        .downcast_ref::<std::io::Error>()
        .and_then(std::io::Error::get_ref)
        .is_some_and(|inner| tls_cause(inner, depth + 1))
    {
        return true;
    }
    error
        .source()
        .is_some_and(|inner| tls_cause(inner, depth + 1))
}

fn cache_header(headers: &HeaderMap, name: HeaderName) -> Option<String> {
    let value = headers.get(name)?.to_str().ok()?;
    (!value.is_empty()
        && value.len() <= MAX_CACHE_HEADER_BYTES
        && !value.bytes().any(|b| b.is_ascii_control()))
    .then(|| value.to_owned())
}

fn redirect_url(
    current: &Url,
    original: &Url,
    headers: &HeaderMap,
    hop: usize,
) -> Result<Url, SourceFailure> {
    if hop >= 3 {
        return Err(failure("redirect_limit", "订阅重定向超过三次限制"));
    }
    let location = headers
        .get(reqwest::header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .filter(|v| v.len() <= MAX_URL_BYTES)
        .ok_or_else(|| failure("redirect", "订阅重定向地址缺失或无效"))?;
    if location.trim() != location || location.bytes().any(|b| b.is_ascii_control() || b == b'\\') {
        return Err(failure("redirect", "订阅重定向地址包含不允许的字符"));
    }
    if location.contains("://") {
        validate_url(location)?;
    }
    if location.strip_prefix("//").is_some_and(|rest| {
        rest.split(['/', '?', '#'])
            .next()
            .is_none_or(|a| a.is_empty() || a.contains('@'))
    }) {
        return Err(failure("redirect", "订阅重定向地址不能携带用户信息"));
    }
    let target = current
        .join(location)
        .map_err(|_| failure("redirect", "订阅重定向地址无效"))?;
    validate_url(target.as_str())?;
    if target.origin() != original.origin() {
        return Err(failure(
            "redirect_origin",
            "订阅重定向到不同来源，须明确更换地址",
        ));
    }
    Ok(target)
}

struct DeadlineReader<'a> {
    inner: Cursor<&'a [u8]>,
    deadline: Instant,
}
impl Read for DeadlineReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if Instant::now() >= self.deadline {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "source decode deadline",
            ));
        }
        self.inner.read(buffer)
    }
}

fn decode_body(
    bytes: Vec<u8>,
    encoding: Option<&str>,
    deadline: Instant,
) -> Result<Vec<u8>, SourceFailure> {
    if bytes.len() > MAX_CONTENT_BYTES {
        return Err(failure("body_limit", "订阅正文超过 2 MiB 限制"));
    }
    if Instant::now() >= deadline {
        return Err(failure("timeout", "订阅获取超过总时间限制"));
    }
    match encoding
        .unwrap_or("identity")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "identity" | "" => Ok(bytes),
        "gzip" => {
            let reader = DeadlineReader {
                inner: Cursor::new(bytes.as_slice()),
                deadline,
            };
            let mut decoder =
                flate2::read::MultiGzDecoder::new(reader).take((MAX_CONTENT_BYTES + 1) as u64);
            let mut body = Vec::new();
            decoder.read_to_end(&mut body).map_err(|_| {
                if Instant::now() >= deadline {
                    failure("timeout", "订阅获取超过总时间限制")
                } else {
                    failure("encoding", "订阅 gzip 正文损坏或不完整")
                }
            })?;
            if body.len() > MAX_CONTENT_BYTES {
                return Err(failure("decompressed_limit", "订阅解压后超过 2 MiB 限制"));
            }
            Ok(body)
        }
        _ => Err(failure("encoding", "订阅使用了不支持的压缩编码")),
    }
}

pub(crate) async fn fetch(config: &FetchConfig) -> Result<FetchOutcome, SourceFailure> {
    let deadline = Instant::now() + FETCH_TIMEOUT;
    fetch_with_deadline(config, &PublicNetwork, deadline).await
}

async fn fetch_with_deadline<N: FetchNetwork>(
    config: &FetchConfig,
    network: &N,
    deadline: Instant,
) -> Result<FetchOutcome, SourceFailure> {
    tokio::time::timeout_at(deadline, fetch_with(config, network, deadline))
        .await
        .map_err(|_| failure("timeout", "订阅获取超过总时间限制"))?
}

async fn fetch_with<N: FetchNetwork>(
    config: &FetchConfig,
    network: &N,
    deadline: Instant,
) -> Result<FetchOutcome, SourceFailure> {
    let original = validate_url(&config.url)?;
    let headers = request_headers(config)?;
    let mut url = original.clone();
    for hop in 0..=3 {
        let host = hostname(&url)?;
        let port = url.port_or_known_default().expect("validated port");
        let addresses = checked_addresses(network.resolve(&host, port).await?, port)?;
        let mut response = network
            .get(&url, &addresses, headers.clone(), deadline)
            .await?;
        if response.status == 304 {
            if config.etag.is_none() && config.last_modified.is_none() {
                return Err(failure("cache", "未建立条件缓存却收到 304 响应"));
            }
            return Ok(FetchOutcome::NotModified {
                etag: cache_header(&response.headers, reqwest::header::ETAG),
                last_modified: cache_header(&response.headers, reqwest::header::LAST_MODIFIED),
            });
        }
        if matches!(response.status, 301 | 302 | 303 | 307 | 308) {
            url = redirect_url(&url, &original, &response.headers, hop)?;
            continue;
        }
        if !(200..300).contains(&response.status) {
            let mut error = match response.status {
                403 => failure("http_403", "订阅服务拒绝访问（403）"),
                429 => failure("http_429", "订阅服务限制请求频率（429）"),
                _ => failure("http", "订阅服务返回非成功 HTTP 状态"),
            };
            error.http_status = Some(response.status);
            return Err(error);
        }
        if let Some(value) = response.headers.get(reqwest::header::CONTENT_LENGTH) {
            let length = value
                .to_str()
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .ok_or_else(|| failure("encoding", "订阅正文长度声明无效"))?;
            if length > MAX_CONTENT_BYTES as u64 {
                return Err(failure("body_limit", "订阅正文超过 2 MiB 限制"));
            }
        }
        if response
            .headers
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| {
                matches!(
                    v.split(';')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_ascii_lowercase()
                        .as_str(),
                    "text/html" | "application/xhtml+xml"
                )
            })
        {
            return Err(failure("non_subscription", "订阅服务返回 HTML 页面"));
        }
        let etag = cache_header(&response.headers, reqwest::header::ETAG);
        let last_modified = cache_header(&response.headers, reqwest::header::LAST_MODIFIED);
        let encoding = response
            .headers
            .get(reqwest::header::CONTENT_ENCODING)
            .map(|v| {
                v.to_str()
                    .map(str::to_owned)
                    .map_err(|_| failure("encoding", "订阅压缩编码无效"))
            })
            .transpose()?;
        let mut bytes = Vec::new();
        while let Some(chunk) = response.body.next().await {
            let chunk = chunk?;
            if chunk.len() > MAX_CONTENT_BYTES.saturating_sub(bytes.len()) {
                return Err(failure("body_limit", "订阅正文超过 2 MiB 限制"));
            }
            bytes.extend_from_slice(&chunk);
        }
        let body = decode_body(bytes, encoding.as_deref(), deadline)?;
        return Ok(FetchOutcome::Modified {
            body,
            etag,
            last_modified,
        });
    }
    Err(failure("redirect_limit", "订阅重定向超过三次限制"))
}

#[cfg(test)]
#[path = "fetch_tests.rs"]
mod tests;
