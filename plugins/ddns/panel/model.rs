use crate::error::{ApiError, ApiResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, PgPool};
use std::{collections::BTreeSet, net::IpAddr};
use uuid::Uuid;

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Config {
    pub name: String,
    pub server_id: i64,
    pub zone_id: String,
    pub record_name: String,
    pub record_type: String,
    pub ttl: u32,
    pub proxied: bool,
    pub interval_secs: u32,
    pub enabled: bool,
    #[serde(default)]
    pub adopt_existing: bool,
}

impl Config {
    pub fn normalize(&mut self) -> ApiResult<()> {
        self.name = self.name.trim().into();
        self.zone_id = self.zone_id.trim().to_ascii_lowercase();
        self.record_name = domain(&self.record_name)
            .ok_or_else(|| ApiError::BadRequest("请输入完整域名，可使用 *.example.com".into()))?;
        if self.name.is_empty() || self.name.len() > 128 || self.name.chars().any(char::is_control)
        {
            return Err(ApiError::BadRequest("名称不能为空或超过 128 字节".into()));
        }
        if !identifier(&self.zone_id) {
            return Err(ApiError::BadRequest(
                "Zone ID 必须为 32 位十六进制字符".into(),
            ));
        }
        if self.server_id <= 0 || !matches!(self.record_type.as_str(), "A" | "AAAA") {
            return Err(ApiError::BadRequest("请选择服务器与 A / AAAA 类型".into()));
        }
        if !(60..=86400).contains(&self.interval_secs) {
            return Err(ApiError::BadRequest("同步间隔必须为 60–86400 秒".into()));
        }
        if self.proxied {
            self.ttl = 1;
        }
        if self.ttl != 1 && !(60..=86400).contains(&self.ttl) {
            return Err(ApiError::BadRequest(
                "TTL 必须为 1（自动）或 60–86400 秒".into(),
            ));
        }
        Ok(())
    }
}

pub(super) fn identifier(value: &str) -> bool {
    value.len() == 32 && value.bytes().all(|c| c.is_ascii_hexdigit())
}

pub(super) fn domain(value: &str) -> Option<String> {
    let value = value.trim().trim_end_matches('.');
    let (prefix, host) = value
        .strip_prefix("*.")
        .map_or(("", value), |host| ("*.", host));
    if host.is_empty()
        || host
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || ":/%?#@\\[]*".contains(c))
    {
        return None;
    }
    let url = reqwest::Url::parse(&format!("https://{host}/")).ok()?;
    let host = url.host_str()?;
    let name = format!("{prefix}{host}");
    if name.len() > 253
        || host.parse::<IpAddr>().is_ok()
        || !host.contains('.')
        || !host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-')
        })
    {
        return None;
    }
    Some(name)
}

pub(super) fn token(value: &str) -> ApiResult<String> {
    let value = value.trim();
    if !(16..=256).contains(&value.len())
        || !value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
    {
        return Err(ApiError::BadRequest(
            "请输入有效的 Cloudflare API Token".into(),
        ));
    }
    Ok(value.into())
}

#[derive(FromRow, Serialize)]
pub(super) struct Rule {
    pub id: Uuid,
    #[sqlx(json)]
    pub config: Config,
    #[serde(skip)]
    pub api_token: String,
    pub revision: i64,
    pub record_id: Option<String>,
    pub last_ip: Option<String>,
    pub last_success_at: Option<i64>,
    pub attempted_at: Option<i64>,
    pub next_run_at: i64,
    pub failures: i32,
    pub status: String,
    pub error_code: Option<String>,
    #[serde(skip)]
    pub lease_until: i64,
}

#[derive(FromRow)]
pub(super) struct Observation {
    pub name: String,
    pub static_info: Value,
    pub static_info_received_at: Option<i64>,
    pub last_seen: Option<i64>,
    pub deleted_at: Option<i64>,
    pub retiring: bool,
    pub plugin_enabled: bool,
}

impl Observation {
    pub fn select(
        &self,
        config: &Config,
        previous: Option<&str>,
        now: i64,
    ) -> Result<IpAddr, &'static str> {
        if !self.plugin_enabled {
            return Err("plugin_disabled");
        }
        if self.deleted_at.is_some() || self.retiring {
            return Err("server_retired");
        }
        if self
            .last_seen
            .is_none_or(|at| at > now + 5 || now.saturating_sub(at) > 60)
        {
            return Err("server_offline");
        }
        if self
            .static_info_received_at
            .is_none_or(|at| at > now + 5 || now.saturating_sub(at) > 600)
        {
            return Err("ip_stale");
        }
        let candidates: BTreeSet<IpAddr> = self.static_info["ip_addresses"]
            .as_array()
            .into_iter()
            .flatten()
            .take(256)
            .filter_map(|value| value.as_str()?.parse::<IpAddr>().ok())
            .filter(|ip| {
                crate::ip_quality::public_ip(*ip) && ip.is_ipv4() == (config.record_type == "A")
            })
            .collect();
        if let Some(previous) = previous
            .and_then(|ip| ip.parse().ok())
            .filter(|ip| candidates.contains(ip))
        {
            return Ok(previous);
        }
        candidates.into_iter().next().ok_or("no_public_ip")
    }
}

pub(super) async fn observation(pool: &PgPool, server_id: i64) -> ApiResult<Observation> {
    sqlx::query_as("SELECT name,static_info,static_info_received_at,last_seen,deleted_at,EXISTS(SELECT 1 FROM server_retirements WHERE server_id=servers.id) AS retiring,EXISTS(SELECT 1 FROM server_plugins WHERE server_id=servers.id AND plugin='ddns' AND enabled) AS plugin_enabled FROM servers WHERE id=$1")
        .bind(server_id).fetch_optional(pool).await?.ok_or(ApiError::NotFound)
}

pub(super) async fn view(pool: &PgPool, rule: Rule) -> ApiResult<Value> {
    let now = sinan_protocol::now_timestamp();
    let info = observation(pool, rule.config.server_id).await?;
    let selected = info.select(&rule.config, rule.last_ip.as_deref(), now);
    let mut value = serde_json::to_value(&rule).map_err(anyhow::Error::from)?;
    value["token_configured"] = (!rule.api_token.is_empty()).into();
    value["plugin_enabled"] = info.plugin_enabled.into();
    value["busy"] = (rule.lease_until > now).into();
    value["server_name"] = info.name.into();
    value["candidate_ip"] = selected.as_ref().ok().map(ToString::to_string).into();
    value["ip_status"] = selected.err().unwrap_or("ready").into();
    value["ip_received_at"] = info.static_info_received_at.into();
    Ok(value)
}
