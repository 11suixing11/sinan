use crate::{
    AppState, auth,
    error::{ApiError, ApiResult},
};
use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use futures_util::{StreamExt, stream};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sinan_protocol::now_timestamp;
use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    net::IpAddr,
    pin::Pin,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

mod cache;
mod errors;
#[cfg(test)]
mod structured_error_tests;
pub use errors::QueryErrorKind;
use errors::{QualityDnsResolver, QueryError};

const PROVIDER_ORIGIN: &str = "https://ipinfo.check.place";
const CACHE_SECS: i64 = 86400;
const RESPONSE_LIMIT: usize = 64 * 1024;
const IP_LIMIT: usize = 8;
const DATABASES: [(&str, &str); 7] = [
    ("maxmind", "MaxMind 地理与 ASN"),
    ("ipapi", "IPAPI"),
    ("scamalytics", "Scamalytics"),
    ("abuseipdb", "AbuseIPDB"),
    ("ip2location", "IP2Location"),
    ("ipdata", "IPData"),
    ("ipqualityscore", "IPQualityScore"),
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QualityField {
    pub label: String,
    pub value: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QualityDatabase {
    pub database: String,
    pub label: String,
    pub status: String,
    pub fields: Vec<QualityField>,
    pub error: Option<String>,
    #[serde(default = "provider_name")]
    pub provider: String,
    #[serde(default)]
    pub target_ip: Option<String>,
    #[serde(default)]
    pub attempted_at: Option<i64>,
    #[serde(default)]
    pub elapsed_ms: Option<u64>,
    #[serde(default)]
    pub error_kind: Option<QueryErrorKind>,
    #[serde(default)]
    pub http_status: Option<u16>,
    #[serde(default)]
    pub last_attempt_at: Option<i64>,
    #[serde(default)]
    pub last_success_at: Option<i64>,
    #[serde(default)]
    pub fresh_until: Option<i64>,
    #[serde(default)]
    pub last_error: Option<QueryFailure>,
    #[serde(default)]
    pub historical: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryFailure {
    pub kind: Option<QueryErrorKind>,
    pub message: String,
    pub http_status: Option<u16>,
    pub attempted_at: Option<i64>,
    pub elapsed_ms: Option<u64>,
}

fn provider_name() -> String {
    "check-place".into()
}

#[derive(Clone, Copy)]
struct QueryAttempt {
    at: i64,
    started: Instant,
}

impl QueryAttempt {
    fn start() -> Self {
        Self {
            at: now_timestamp(),
            started: Instant::now(),
        }
    }

    fn elapsed_ms(self) -> u64 {
        self.started
            .elapsed()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IpQuality {
    pub ip: String,
    pub checked_at: i64,
    pub expires_at: i64,
    pub status: String,
    pub databases: Vec<QualityDatabase>,
    #[serde(default = "provider_name")]
    pub provider: String,
    #[serde(default)]
    pub last_attempt_at: Option<i64>,
    #[serde(default)]
    pub last_success_at: Option<i64>,
    #[serde(default)]
    pub fresh_until: Option<i64>,
    #[serde(default)]
    pub last_error: BTreeMap<String, QueryFailure>,
}

type DatabaseRequest = Pin<Box<dyn Future<Output = (String, QualityDatabase)> + Send>>;

pub fn reported_ips(info: &Value) -> Vec<String> {
    let mut ips: Vec<_> = info["ip_addresses"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str()?.parse::<IpAddr>().ok())
        .map(|ip| ip.to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    ips.sort_by(|a, b| {
        let a_public = a.parse::<IpAddr>().is_ok_and(public_ip);
        let b_public = b.parse::<IpAddr>().is_ok_and(public_ip);
        b_public.cmp(&a_public).then_with(|| a.cmp(b))
    });
    ips.truncate(IP_LIMIT);
    ips
}

pub fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !matches!(a, 0 | 10 | 127 | 224..=255)
                && !(a == 100 && (64..=127).contains(&b))
                && !(a == 169 && b == 254)
                && !(a == 172 && (16..=31).contains(&b))
                && !(a == 192 && ((b == 0 && matches!(c, 0 | 2)) || b == 168))
                && !(a == 198 && (matches!(b, 18 | 19) || (b == 51 && c == 100)))
                && !(a == 203 && b == 0 && c == 113)
        }
        IpAddr::V6(ip) => {
            let segments = ip.segments();
            segments[0] & 0xe000 == 0x2000 && !(segments[0] == 0x2001 && segments[1] == 0x0db8)
        }
    }
}

pub async fn cached(state: &AppState, server_id: i64, ips: &[String]) -> ApiResult<Vec<IpQuality>> {
    cache::read(&state.pool, server_id, ips).await
}

pub async fn refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> ApiResult<Json<Vec<IpQuality>>> {
    auth::require_admin(&state, &headers).await?;
    let _permit = state
        .quality_permits
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError::Busy)?;
    let now = now_timestamp();
    let ips = cache::begin_refresh(&state.pool, id, now).await?;
    let result = refresh_inner(&state, id, &ips).await;
    sqlx::query("UPDATE servers SET quality_refresh_started=NULL WHERE id=$1 AND quality_refresh_started=$2")
        .bind(id).bind(now).execute(&state.pool).await?;
    result.map(Json)
}

async fn refresh_inner(state: &AppState, id: i64, ips: &[String]) -> ApiResult<Vec<IpQuality>> {
    let client = Client::builder()
        .dns_resolver(Arc::new(QualityDnsResolver))
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(6))
        .build()
        .map_err(anyhow::Error::from)?;
    let mut quality = query_all(&client, PROVIDER_ORIGIN, ips, false).await;
    quality.sort_by(|a, b| a.ip.cmp(&b.ip));
    cache::persist(&state.pool, id, &quality).await?;
    cache::read(&state.pool, id, ips).await
}

async fn query_all(
    client: &Client,
    origin: &str,
    ips: &[String],
    allow_documentation_ips: bool,
) -> Vec<IpQuality> {
    query_all_with_limit(
        client,
        origin,
        ips,
        allow_documentation_ips,
        Duration::from_secs(40),
    )
    .await
}

async fn query_all_with_limit(
    client: &Client,
    origin: &str,
    ips: &[String],
    allow_documentation_ips: bool,
    total_limit: Duration,
) -> Vec<IpQuality> {
    let now = now_timestamp();
    let attempts = Arc::new(Mutex::new(BTreeMap::new()));
    let mut requests: Vec<DatabaseRequest> = Vec::new();
    for ip in ips {
        for (database, label) in DATABASES {
            let client = client.clone();
            let origin = origin.to_owned();
            let ip = ip.clone();
            let attempts = attempts.clone();
            requests.push(Box::pin(async move {
                let attempt = QueryAttempt::start();
                attempts
                    .lock()
                    .expect("query attempt lock")
                    .insert((ip.clone(), database.to_owned()), attempt);
                let result = if ip.parse::<IpAddr>().is_ok_and(|address| {
                    public_ip(address) || (allow_documentation_ips && documentation_ip(address))
                }) {
                    query_database(&client, &origin, &ip, database).await
                } else {
                    Err(QueryError::new(
                        QueryErrorKind::NotPublic,
                        "此地址不属于公网单播 IP，未向第三方查询",
                    ))
                };
                let entry = database_result(database, label, &ip, Some(attempt), result);
                (ip, entry)
            }));
        }
    }
    let mut pending = stream::iter(requests).buffer_unordered(4);
    let mut results = Vec::new();
    let deadline = tokio::time::Instant::now() + total_limit;
    while let Ok(Some(result)) = tokio::time::timeout_at(deadline, pending.next()).await {
        results.push(result);
    }
    ips.iter()
        .map(|ip| {
            let databases: Vec<_> = DATABASES
                .iter()
                .map(|&(database, label)| {
                    results
                        .iter()
                        .find(|(address, entry)| address == ip && entry.database == database)
                        .map(|(_, entry)| entry.clone())
                        .unwrap_or_else(|| {
                            let attempt = attempts
                                .lock()
                                .expect("query attempt lock")
                                .get(&(ip.clone(), database.to_owned()))
                                .copied();
                            database_result(
                                database,
                                label,
                                ip,
                                attempt,
                                Err(QueryError::new(
                                    if attempt.is_some() {
                                        QueryErrorKind::Timeout
                                    } else {
                                        QueryErrorKind::NotAttempted
                                    },
                                    if attempt.is_some() {
                                        "质量查询超过总时间限制"
                                    } else {
                                        "查询批次超过总时间限制，此数据库尚未开始查询"
                                    },
                                )),
                            )
                        })
                })
                .collect();
            let succeeded = databases
                .iter()
                .filter(|entry| entry.status == "succeeded")
                .count();
            let status = match succeeded {
                0 => "failed",
                7 => "succeeded",
                _ => "partial",
            }
            .into();
            IpQuality {
                ip: ip.clone(),
                checked_at: now,
                expires_at: now + CACHE_SECS,
                status,
                databases,
                provider: provider_name(),
                last_attempt_at: Some(now),
                last_success_at: None,
                fresh_until: None,
                last_error: BTreeMap::new(),
            }
        })
        .collect()
}

fn documentation_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            matches!((a, b, c), (192, 0, 2) | (198, 51, 100) | (203, 0, 113))
        }
        IpAddr::V6(ip) => ip.segments()[0] == 0x2001 && ip.segments()[1] == 0x0db8,
    }
}

fn database_result(
    database: &str,
    label: &str,
    ip: &str,
    attempt: Option<QueryAttempt>,
    result: Result<Vec<QualityField>, QueryError>,
) -> QualityDatabase {
    let (status, fields, error, error_kind, http_status) = match result {
        Ok(fields) => ("succeeded", fields, None, None, None),
        Err(error) => (
            "failed",
            Vec::new(),
            Some(error.to_string()),
            Some(error.kind),
            error.http_status,
        ),
    };
    let success_at = (status == "succeeded").then(now_timestamp);
    QualityDatabase {
        database: database.into(),
        label: label.into(),
        status: status.into(),
        fields,
        error,
        provider: provider_name(),
        target_ip: Some(ip.into()),
        attempted_at: attempt.map(|attempt| attempt.at),
        elapsed_ms: attempt.map(QueryAttempt::elapsed_ms),
        error_kind,
        http_status,
        last_attempt_at: attempt.map(|attempt| attempt.at),
        last_success_at: success_at,
        fresh_until: success_at.map(|at| at.saturating_add(CACHE_SECS)),
        last_error: None,
        historical: false,
    }
}

async fn query_database(
    client: &Client,
    origin: &str,
    ip: &str,
    database: &str,
) -> Result<Vec<QualityField>, QueryError> {
    let mut url = reqwest::Url::parse(origin)
        .map_err(|_| QueryError::new(QueryErrorKind::InvalidOrigin, "质量查询服务地址无效"))?;
    url.set_path(&format!("/{ip}"));
    let request = client.get(url).query(&[("lang", "cn")]);
    let request = if database == "maxmind" {
        request
    } else {
        request.query(&[("db", database)])
    };
    let mut response = request
        .send()
        .await
        .map_err(|error| QueryError::request(&error, false))?;
    if !response.status().is_success() {
        return Err(QueryError::http(response.status().as_u16()));
    }
    if response
        .content_length()
        .is_some_and(|bytes| bytes > RESPONSE_LIMIT as u64)
    {
        return Err(QueryError::new(
            QueryErrorKind::ResponseLimit,
            "质量查询响应超过 64 KiB",
        ));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| QueryError::request(&error, true))?
    {
        if body.len() + chunk.len() > RESPONSE_LIMIT {
            return Err(QueryError::new(
                QueryErrorKind::ResponseLimit,
                "质量查询响应超过 64 KiB",
            ));
        }
        body.extend_from_slice(&chunk);
    }
    let value: Value = serde_json::from_slice(&body)
        .map_err(|_| QueryError::new(QueryErrorKind::NonJson, "质量查询返回的内容不是有效 JSON"))?;
    let fields = parse_fields(database, &value);
    if fields.is_empty() {
        return Err(QueryError::new(
            QueryErrorKind::SchemaMismatch,
            "质量查询响应缺少已知字段，此数据库信息未知",
        ));
    }
    Ok(fields)
}

fn parse_fields(database: &str, value: &Value) -> Vec<QualityField> {
    let mappings: &[(&str, &str)] = match database {
        "maxmind" => &[
            ("/ASN/AutonomousSystemNumber", "ASN"),
            ("/ASN/AutonomousSystemOrganization", "网络组织"),
            ("/Country/Name", "国家或地区"),
            ("/Country/IsoCode", "国家代码"),
            ("/City/Name", "城市"),
            ("/City/Latitude", "纬度"),
            ("/City/Longitude", "经度"),
            ("/City/Location/TimeZone", "时区"),
        ],
        "ipapi" => &[
            ("/asn/type", "ASN 类型"),
            ("/company/type", "组织类型"),
            ("/company/abuser_score", "滥用评分（上游原值）"),
            ("/location/country_code", "国家代码"),
            ("/is_proxy", "代理"),
            ("/is_tor", "Tor"),
            ("/is_vpn", "VPN"),
            ("/is_datacenter", "数据中心"),
            ("/is_abuser", "滥用"),
            ("/is_crawler", "爬虫"),
        ],
        "scamalytics" => &[
            ("/scamalytics/scamalytics_score", "风险评分（上游原值）"),
            ("/scamalytics/scamalytics_proxy/is_vpn", "VPN"),
            ("/scamalytics/scamalytics_proxy/is_datacenter", "数据中心"),
            ("/scamalytics/is_blacklisted_external", "外部黑名单"),
            ("/external_datasources/firehol/is_proxy", "FireHOL 代理"),
            ("/external_datasources/x4bnet/is_tor", "X4B Tor"),
            (
                "/external_datasources/maxmind_geolite2/ip_country_code",
                "国家代码",
            ),
        ],
        "abuseipdb" => &[
            ("/data/usageType", "用途类型"),
            ("/data/abuseConfidenceScore", "滥用置信度（上游原值）"),
        ],
        "ip2location" => &[
            ("/fraud_score", "欺诈评分（上游原值）"),
            ("/country_code", "国家代码"),
            ("/usage_type", "用途类型"),
            ("/as_info/as_usage_type", "ASN 用途"),
            ("/is_proxy", "代理"),
            ("/proxy/is_public_proxy", "公共代理"),
            ("/proxy/is_web_proxy", "网页代理"),
            ("/proxy/is_tor", "Tor"),
            ("/proxy/is_vpn", "VPN"),
            ("/proxy/is_data_center", "数据中心"),
            ("/proxy/is_spammer", "垃圾邮件"),
            ("/proxy/is_web_crawler", "爬虫"),
            ("/proxy/is_scanner", "扫描器"),
            ("/proxy/is_botnet", "僵尸网络"),
        ],
        "ipdata" => &[
            ("/country_code", "国家代码"),
            ("/threat/is_proxy", "代理"),
            ("/threat/is_tor", "Tor"),
            ("/threat/is_datacenter", "数据中心"),
            ("/threat/is_threat", "威胁"),
            ("/threat/is_known_abuser", "已知滥用"),
            ("/threat/is_known_attacker", "已知攻击者"),
        ],
        "ipqualityscore" => &[
            ("/fraud_score", "欺诈评分（上游原值）"),
            ("/country_code", "国家代码"),
            ("/proxy", "代理"),
            ("/tor", "Tor"),
            ("/vpn", "VPN"),
            ("/recent_abuse", "近期滥用"),
            ("/bot_status", "机器人"),
        ],
        _ => &[],
    };
    mappings
        .iter()
        .filter_map(|(path, label)| {
            let value = value.pointer(path)?;
            let value = match value {
                Value::String(value) => Value::String(value.chars().take(512).collect()),
                Value::Number(_) | Value::Bool(_) => value.clone(),
                _ => return None,
            };
            Some(QualityField {
                label: (*label).into(),
                value,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, http::StatusCode, response::IntoResponse, routing::get};
    use serde_json::json;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[test]
    fn fields_preserve_false_zero_and_unknown_without_inference() {
        let fields = parse_fields("ipqualityscore", &json!({"fraud_score":0,"proxy":false}));
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].value, json!(0));
        assert_eq!(fields[1].value, json!(false));
        assert!(parse_fields("ipqualityscore", &json!({"error":"unavailable"})).is_empty());
        assert_eq!(
            parse_fields(
                "ipapi",
                &json!({"company":{"abuser_score":"0.0047 (Very Low)"}})
            )[0]
            .value,
            json!("0.0047 (Very Low)")
        );
    }

    #[test]
    fn only_routable_addresses_are_sent_to_the_provider() {
        for value in [
            "127.0.0.1",
            "10.0.0.1",
            "100.64.0.1",
            "192.0.2.1",
            "198.18.0.1",
            "203.0.113.1",
            "224.0.0.1",
            "::1",
            "fe80::1",
            "fc00::1",
            "2001:db8::1",
            "::ffff:127.0.0.1",
        ] {
            assert!(!public_ip(value.parse().unwrap()), "{value}");
        }
    }

    #[tokio::test]
    async fn providers_fail_independently_and_redirects_are_not_followed() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let followed = Arc::new(AtomicUsize::new(0));
        let redirect_count = followed.clone();
        let app = Router::new().route("/{ip}", get(|axum::extract::Query(query): axum::extract::Query<std::collections::HashMap<String, String>>| async move {
            match query.get("db").map(String::as_str) {
                None => (StatusCode::OK, Json(json!({"ASN":{"AutonomousSystemNumber":64500},"Country":{"Name":"示例"}}))).into_response(),
                Some("ipqualityscore") => (StatusCode::OK, Json(json!({"fraud_score":0,"proxy":false}))).into_response(),
                Some("ipapi") => (StatusCode::FOUND, [("location", "/unexpected-redirect")], "").into_response(),
                _ => (StatusCode::FORBIDDEN, Json(json!({"error":"unavailable"}))).into_response(),
            }
        })).route("/unexpected-redirect", get(move || { let count = redirect_count.clone(); async move { count.fetch_add(1, Ordering::SeqCst); "unexpected" } }));
        let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let client = Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        let quality = query_all(
            &client,
            &format!("http://{address}"),
            &["192.0.2.1".into()],
            true,
        )
        .await;
        task.abort();
        assert_eq!(followed.load(Ordering::SeqCst), 0);
        assert_eq!(quality[0].status, "partial");
        assert_eq!(
            quality[0]
                .databases
                .iter()
                .filter(|database| database.status == "succeeded")
                .count(),
            2
        );
        assert!(
            quality[0].databases[1]
                .error
                .as_deref()
                .unwrap()
                .contains("302")
        );
        assert!(quality[0].databases[2].fields.is_empty());
        assert!(
            quality[0].databases[2]
                .error
                .as_deref()
                .unwrap()
                .contains("403")
        );
    }

    #[tokio::test]
    async fn provider_responses_are_limited_by_size_and_time() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = Router::new().route(
            "/{ip}",
            get(
                |axum::extract::Query(query): axum::extract::Query<
                    std::collections::HashMap<String, String>,
                >| async move {
                    if query.get("db").is_some_and(|value| value == "ipapi") {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                    "x".repeat(RESPONSE_LIMIT + 1)
                },
            ),
        );
        let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let client = Client::builder()
            .no_proxy()
            .timeout(Duration::from_millis(50))
            .build()
            .unwrap();
        let base = format!("http://{address}");
        let oversized = query_database(&client, &base, "192.0.2.1", "maxmind")
            .await
            .unwrap_err();
        assert_eq!(oversized.kind, QueryErrorKind::ResponseLimit);
        assert!(oversized.to_string().contains("64 KiB"));
        let timed_out = query_database(&client, &base, "192.0.2.1", "ipapi")
            .await
            .unwrap_err();
        assert_eq!(timed_out.kind, QueryErrorKind::Timeout);
        assert!(timed_out.to_string().contains("超时"));
        task.abort();
    }
}
