use super::parse::{ImportError, MAX_BODY};
use flate2::read::{MultiGzDecoder, ZlibDecoder};
use reqwest::{Client, Url, header, redirect::Policy};
use std::{
    io::Read,
    net::{IpAddr, SocketAddr},
    time::Duration,
};

pub(super) struct FetchResult {
    pub body: Option<Vec<u8>>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub traffic: Option<serde_json::Value>,
}

pub(super) const DEFAULT_USER_AGENT: &str = "Sinan-subscription-import/1";

pub(super) fn validate_user_agent(value: &str) -> Result<(), ImportError> {
    if value.is_empty()
        || value.len() > 256
        || !value.is_ascii()
        || value.bytes().any(|byte| !(32..127).contains(&byte))
        || value.trim().is_empty()
    {
        Err(ImportError("invalid_source_user_agent"))
    } else {
        Ok(())
    }
}

pub(super) fn parse_traffic(value: &str) -> Option<serde_json::Value> {
    if value.len() > 4096 {
        return None;
    }
    let mut fields = serde_json::Map::new();
    let mut seen = std::collections::BTreeSet::new();
    for field in value.split(';') {
        let Some((key, value)) = field.trim().split_once('=') else {
            continue;
        };
        let key = key.trim();
        if !matches!(key, "upload" | "download" | "total" | "expire") {
            continue;
        }
        let value = value.trim();
        if !seen.insert(key) || value.is_empty() || !value.bytes().all(|v| v.is_ascii_digit()) {
            return None;
        }
        let number = value.parse::<i64>().ok()?;
        if key == "expire" && number == 0 {
            continue;
        }
        fields.insert(key.into(), number.into());
    }
    (!fields.is_empty()).then_some(serde_json::Value::Object(fields))
}

pub(super) fn validate_url(value: &str) -> Result<Url, ImportError> {
    if value.len() > 8192 || value.contains('\\') || value.chars().any(char::is_control) {
        return Err(ImportError("invalid_source_url"));
    }
    let url = Url::parse(value).map_err(|_| ImportError("invalid_source_url"))?;
    if url.scheme() != "https"
        || has_userinfo(value)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.host_str().is_none()
        || url.port_or_known_default().is_none()
    {
        return Err(ImportError("invalid_source_url"));
    }
    let host = url.host_str().expect("host").trim_matches(['[', ']']);
    if host.eq_ignore_ascii_case("localhost")
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || host.ends_with(".internal")
        || host.parse::<IpAddr>().is_ok_and(|ip| !public_address(ip))
    {
        return Err(ImportError("non_public_source_address"));
    }
    Ok(url)
}

fn has_userinfo(value: &str) -> bool {
    value
        .split_once("://")
        .map(|(_, tail)| tail)
        .or_else(|| value.strip_prefix("//"))
        .and_then(|tail| tail.split(['/', '?', '#']).next())
        .is_some_and(|authority| authority.contains('@'))
}

pub(super) fn validate_authorization(value: &str) -> Result<(), ImportError> {
    if value.is_empty() || value.len() > 8192 || header::HeaderValue::from_str(value).is_err() {
        Err(ImportError("invalid_source_authorization"))
    } else {
        Ok(())
    }
}

pub(super) async fn download(
    value: &str,
    authorization: Option<&str>,
    etag: Option<&str>,
    last_modified: Option<&str>,
    user_agent: &str,
) -> Result<FetchResult, ImportError> {
    validate_user_agent(user_agent)?;
    tokio::time::timeout(
        Duration::from_secs(20),
        download_inner(value, authorization, etag, last_modified, user_agent),
    )
    .await
    .map_err(|_| ImportError("download_timeout"))?
}

async fn download_inner(
    value: &str,
    authorization: Option<&str>,
    etag: Option<&str>,
    last_modified: Option<&str>,
    user_agent: &str,
) -> Result<FetchResult, ImportError> {
    let origin = validate_url(value)?;
    if let Some(authorization) = authorization {
        validate_authorization(authorization)?;
    }
    let mut url = origin.clone();
    for hop in 0..=3 {
        validate_target(&origin, &url)?;
        let host = url.host_str().expect("host").trim_matches(['[', ']']);
        let port = url.port_or_known_default().expect("port");
        let addresses: Vec<SocketAddr> = match host.parse::<IpAddr>() {
            Ok(ip) => vec![SocketAddr::new(ip, port)],
            Err(_) => tokio::net::lookup_host((host, port))
                .await
                .map_err(|_| ImportError("source_dns_failed"))?
                .take(65)
                .collect(),
        };
        if addresses.is_empty()
            || addresses.len() > 64
            || addresses
                .iter()
                .any(|address| !public_address(address.ip()))
        {
            return Err(ImportError("non_public_source_address"));
        }
        let client = Client::builder()
            .no_proxy()
            .https_only(true)
            .redirect(Policy::none())
            .referer(false)
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .no_zstd()
            .resolve_to_addrs(host, &addresses)
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(20))
            .user_agent(user_agent)
            .build()
            .map_err(|_| ImportError("download_client_failed"))?;
        let request = build_request(&client, &origin, &url, authorization, etag, last_modified)?;
        let mut response = client
            .execute(request)
            .await
            .map_err(|_| ImportError("download_failed"))?;
        if response.status() == reqwest::StatusCode::NOT_MODIFIED {
            if etag.is_none() && last_modified.is_none() {
                return Err(ImportError("unexpected_not_modified"));
            }
            return Ok(FetchResult {
                body: None,
                etag: response_header(&response, header::ETAG),
                last_modified: response_header(&response, header::LAST_MODIFIED),
                traffic: response_header(
                    &response,
                    header::HeaderName::from_static("subscription-userinfo"),
                )
                .and_then(|value| parse_traffic(&value)),
            });
        }
        if response.status().is_redirection() {
            url = redirect_target(&origin, &url, response.headers(), hop)?;
            continue;
        }
        if !response.status().is_success() {
            return Err(ImportError("source_http_failed"));
        }
        if response
            .content_length()
            .is_some_and(|v| v > MAX_BODY as u64)
        {
            return Err(ImportError("body_limit"));
        }
        if response_header(&response, header::CONTENT_TYPE)
            .is_some_and(|v| v.to_ascii_lowercase().contains("text/html"))
        {
            return Err(ImportError("html_response"));
        }
        let encoding = response_header(&response, header::CONTENT_ENCODING).unwrap_or_default();
        let etag = response_header(&response, header::ETAG);
        let last_modified = response_header(&response, header::LAST_MODIFIED);
        let traffic = response_header(
            &response,
            header::HeaderName::from_static("subscription-userinfo"),
        )
        .and_then(|value| parse_traffic(&value));
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| ImportError("download_failed"))?
        {
            if chunk.len() > MAX_BODY.saturating_sub(body.len()) {
                return Err(ImportError("body_limit"));
            }
            body.extend_from_slice(&chunk);
        }
        let body = decode_body(&body, &encoding)?;
        return Ok(FetchResult {
            body: Some(body),
            etag,
            last_modified,
            traffic,
        });
    }
    Err(ImportError("redirect_limit"))
}

fn validate_target(origin: &Url, target: &Url) -> Result<(), ImportError> {
    validate_url(target.as_str())?;
    if target.origin() != origin.origin() {
        return Err(ImportError("cross_origin_redirect"));
    }
    Ok(())
}

fn build_request(
    client: &Client,
    origin: &Url,
    target: &Url,
    authorization: Option<&str>,
    etag: Option<&str>,
    last_modified: Option<&str>,
) -> Result<reqwest::Request, ImportError> {
    // Validate again at the credential attachment boundary, even if a future
    // caller obtains a target without using the redirect parser.
    validate_target(origin, target)?;
    let mut request = client
        .get(target.clone())
        .header(header::ACCEPT_ENCODING, "gzip, deflate");
    if let Some(value) = authorization {
        request = request.header(header::AUTHORIZATION, value);
    }
    if let Some(value) = etag {
        request = request.header(header::IF_NONE_MATCH, value);
    }
    if let Some(value) = last_modified {
        request = request.header(header::IF_MODIFIED_SINCE, value);
    }
    request
        .build()
        .map_err(|_| ImportError("invalid_source_request"))
}

fn redirect_target(
    origin: &Url,
    current: &Url,
    headers: &header::HeaderMap,
    hop: usize,
) -> Result<Url, ImportError> {
    if hop >= 3 {
        return Err(ImportError("redirect_limit"));
    }
    let location = headers
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .filter(|value| {
            value.len() <= 8192
                && !value.contains('\\')
                && !has_userinfo(value)
                && !value.chars().any(char::is_control)
        })
        .ok_or(ImportError("invalid_redirect"))?;
    let target = current
        .join(location)
        .map_err(|_| ImportError("invalid_redirect"))?;
    validate_target(origin, &target)?;
    Ok(target)
}

fn response_header(response: &reqwest::Response, key: header::HeaderName) -> Option<String> {
    response
        .headers()
        .get(key)
        .and_then(|v| v.to_str().ok())
        .filter(|v| v.len() <= 8192)
        .map(str::to_owned)
}

fn decode_body(bytes: &[u8], encoding: &str) -> Result<Vec<u8>, ImportError> {
    let reader: Box<dyn Read + '_> = match encoding.trim().to_ascii_lowercase().as_str() {
        "" | "identity" => Box::new(bytes),
        "gzip" => Box::new(MultiGzDecoder::new(bytes)),
        "deflate" => Box::new(ZlibDecoder::new(bytes)),
        _ => return Err(ImportError("unsupported_content_encoding")),
    };
    let mut body = Vec::new();
    reader
        .take((MAX_BODY + 1) as u64)
        .read_to_end(&mut body)
        .map_err(|_| ImportError("invalid_compressed_body"))?;
    if body.len() > MAX_BODY {
        return Err(ImportError("decompressed_body_limit"));
    }
    Ok(body)
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
            // Only native global unicast. Transition, embedded IPv4, special-use,
            // documentation and local ranges cannot bypass IPv4 policy.
            s[0] & 0xe000 == 0x2000
                && s[0] != 0x2002
                && s[0] != 0x3fff
                && !(s[0] == 0x2001 && (s[1] <= 0x01ff || s[1] == 0x0db8))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscription_traffic_is_partial_bounded_and_never_guesses_missing_fields() {
        assert_eq!(
            parse_traffic("upload=12; download=30; total=100; expire=2000000000"),
            Some(
                serde_json::json!({"upload":12,"download":30,"total":100,"expire":2000000000_i64})
            )
        );
        assert_eq!(
            parse_traffic("total=0; expire=0; unknown=value"),
            Some(serde_json::json!({"total":0}))
        );
        for invalid in [
            "upload=-1",
            "total=1.5",
            "total=9223372036854775808",
            "download=1; download=2",
            "expire=0; expire=1",
            "unknown=value",
            "upload=+1",
        ] {
            assert!(parse_traffic(invalid).is_none(), "{invalid}");
        }
        assert!(parse_traffic(&" ".repeat(4097)).is_none());
    }

    #[test]
    fn source_user_agent_cannot_inject_headers_or_grow_unbounded() {
        for valid in [
            DEFAULT_USER_AGENT,
            "sing-box/1.14.2",
            "Client (test; compat)",
        ] {
            assert!(validate_user_agent(valid).is_ok());
        }
        for invalid in [
            "",
            " ",
            "agent\r\nAuthorization: secret",
            "agent\tvalue",
            "中文",
            "agent\u{7f}",
        ] {
            assert!(validate_user_agent(invalid).is_err());
        }
        assert!(validate_user_agent(&"a".repeat(257)).is_err());
    }
    use std::io::Write;

    #[test]
    fn rejects_private_embedded_and_reserved_targets() {
        for host in [
            "127.0.0.1",
            "0.0.0.0",
            "10.1.2.3",
            "172.16.0.1",
            "192.168.0.1",
            "169.254.169.254",
            "100.64.0.1",
            "198.18.0.1",
            "192.0.2.1",
            "::",
            "::1",
            "::ffff:127.0.0.1",
            "::ffff:169.254.169.254",
            "::ffff:0:7f00:1",
            "::7f00:1",
            "64:ff9b::7f00:1",
            "64:ff9b::a9fe:a9fe",
            "64:ff9b:1::a00:1",
            "2002:7f00:1::",
            "2002:a9fe:a9fe::",
            "2001::1",
            "2001:db8::1",
            "3fff::1",
            "fe80::1",
            "fd00::1",
        ] {
            assert!(!public_address(host.parse().unwrap()), "{host}");
        }
        for host in ["2003::1", "2600::1", "2a00::1"] {
            assert!(public_address(host.parse().unwrap()), "{host}");
            assert!(validate_url(&format!("https://[{host}]/subscription")).is_ok());
        }
        for url in [
            "http://example.com",
            "file:///tmp/example",
            "https://user:secret@example.com",
            "https://@example.com",
            "https://localhost",
            "https://2130706433",
            "https://0x7f000001",
            "https://127.1",
        ] {
            assert!(validate_url(url).is_err());
        }
    }

    #[test]
    fn rejects_decompression_expansion_and_unknown_encodings() {
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        encoder.write_all(&vec![b'x'; MAX_BODY + 1]).unwrap();
        let bytes = encoder.finish().unwrap();
        assert!(bytes.len() < MAX_BODY);
        assert!(decode_body(&bytes, "gzip").is_err());
        let mut prefix = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        prefix.write_all(b"first-member").unwrap();
        let prefix = prefix.finish().unwrap();
        assert_eq!(
            decode_body(&[prefix.as_slice(), prefix.as_slice()].concat(), "gzip").unwrap(),
            b"first-memberfirst-member"
        );
        assert!(decode_body(&[prefix, bytes].concat(), "gzip").is_err());
        assert!(decode_body(b"x", "br").is_err());
        assert_eq!(decode_body(b"fixture", "identity").unwrap(), b"fixture");
    }

    #[test]
    fn temporary_redirects_cannot_attach_credentials_to_a_different_origin() {
        let origin = validate_url("https://source.example.com/subscription?token=fixture").unwrap();
        let client = Client::builder()
            .no_proxy()
            .redirect(Policy::none())
            .build()
            .unwrap();
        let request = build_request(
            &client,
            &origin,
            &origin,
            Some("Bearer fixture-secret"),
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            request.headers()[header::AUTHORIZATION],
            "Bearer fixture-secret"
        );
        let response: reqwest::Response = axum::http::Response::builder()
            .status(reqwest::StatusCode::TEMPORARY_REDIRECT)
            .header(header::LOCATION, "https://other.example.com/collect")
            .body(String::new())
            .unwrap()
            .into();
        assert!(response.status().is_redirection());
        assert_eq!(
            redirect_target(&origin, &origin, response.headers(), 0)
                .unwrap_err()
                .0,
            "cross_origin_redirect"
        );
        let cross_origin = Url::parse("https://other.example.com/collect").unwrap();
        assert_eq!(
            build_request(
                &client,
                &origin,
                &cross_origin,
                Some("Bearer fixture-secret"),
                None,
                None
            )
            .unwrap_err()
            .0,
            "cross_origin_redirect"
        );

        let mut headers = header::HeaderMap::new();
        headers.insert(header::LOCATION, header::HeaderValue::from_static("/next"));
        let next = redirect_target(&origin, &origin, &headers, 2).unwrap();
        assert_eq!(next.as_str(), "https://source.example.com/next");
        assert!(
            build_request(
                &client,
                &origin,
                &next,
                Some("Bearer fixture-secret"),
                None,
                None
            )
            .is_ok()
        );
        assert_eq!(
            redirect_target(&origin, &origin, &headers, 3)
                .unwrap_err()
                .0,
            "redirect_limit"
        );
    }

    #[test]
    fn redirects_reject_private_targets_and_malformed_locations_before_dns() {
        let origin = validate_url("https://source.example.com/subscription").unwrap();
        for location in [
            "https://127.1/private",
            "//2130706433/private",
            "https://0x7f000001/private",
            "https://[::ffff:127.0.0.1]/private",
            "https://localhost/private",
            "https://source.example.com:444/private",
            "http://source.example.com/private",
            "https://credential@source.example.com/private",
            "https://@source.example.com/private",
            "//@source.example.com/private",
            "https:\\source.example.com\\private",
            "https://source.example.com/#secret",
            "https://[invalid",
            "\thttps://source.example.com/private",
        ] {
            let mut headers = header::HeaderMap::new();
            headers.insert(
                header::LOCATION,
                header::HeaderValue::from_str(location).unwrap(),
            );
            assert!(
                redirect_target(&origin, &origin, &headers, 0).is_err(),
                "{location}"
            );
        }
    }
}
