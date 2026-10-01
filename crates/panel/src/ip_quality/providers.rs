use super::{DATABASES, IpQuality, QualityField, QueryError, QueryErrorKind};
use reqwest::{
    Client,
    header::{ACCEPT, HeaderValue},
};
use serde::Serialize;
use serde_json::Value;
use std::net::IpAddr;

const ABUSEIPDB_ENDPOINT: &str = "https://api.abuseipdb.com/api/v2/check";
const OFFICIAL_DATABASES: [(&str, &str); 1] = [("abuseipdb-v2", "AbuseIPDB 官方 IP 查询")];

#[derive(Clone, Serialize)]
pub struct ProviderDescription {
    pub provider: String,
    pub label: String,
    pub kind: String,
    pub execution: String,
    pub enabled: bool,
    pub reason: Option<String>,
    pub databases: Vec<DatabaseDescription>,
}

#[derive(Clone, Serialize)]
pub struct DatabaseDescription {
    pub database: String,
    pub label: String,
}

#[derive(Clone)]
enum Adapter {
    CheckPlace { origin: String },
    AbuseIpDb { endpoint: String, key: HeaderValue },
}

#[derive(Clone)]
pub(super) struct Provider {
    id: &'static str,
    label: &'static str,
    kind: &'static str,
    execution: &'static str,
    adapter: Option<Adapter>,
    reason: Option<String>,
    databases: &'static [(&'static str, &'static str)],
}

#[derive(Clone)]
pub struct ProviderRegistry {
    providers: Vec<Provider>,
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::configured(None)
    }
}

impl ProviderRegistry {
    pub fn from_env() -> Self {
        match std::env::var("SINAN_ABUSEIPDB_API_KEY") {
            Ok(key) => Self::configured(Some(&key)),
            Err(std::env::VarError::NotPresent) => Self::default(),
            Err(std::env::VarError::NotUnicode(_)) => Self::configured(Some("\0")),
        }
    }

    fn configured(key: Option<&str>) -> Self {
        let credential = key
            .filter(|key| {
                !key.is_empty()
                    && key.len() <= 1024
                    && key.bytes().all(|byte| byte.is_ascii_graphic())
            })
            .and_then(|key| HeaderValue::from_str(key).ok())
            .map(|mut key| {
                key.set_sensitive(true);
                key
            });
        let reason = credential.is_none().then(|| {
            if key.is_none_or(|key| key.trim().is_empty()) {
                "未配置 SINAN_ABUSEIPDB_API_KEY，正式接口未启用，信息未知".into()
            } else {
                "SINAN_ABUSEIPDB_API_KEY 格式无效，正式接口未启用，信息未知".into()
            }
        });
        Self { providers: vec![
            Provider { id: "check-place", label: "check-place 聚合入口", kind: "aggregator", execution: "panel", adapter: Some(Adapter::CheckPlace { origin: super::PROVIDER_ORIGIN.into() }), reason: None, databases: &DATABASES },
            Provider { id: "abuseipdb-api", label: "AbuseIPDB 官方接口", kind: "credential_api", execution: "panel", adapter: credential.map(|key| Adapter::AbuseIpDb { endpoint: ABUSEIPDB_ENDPOINT.into(), key }), reason, databases: &OFFICIAL_DATABASES },
            Provider { id: "ipquality-node", label: "IPQuality 节点自查", kind: "node_self", execution: "node", adapter: None, reason: Some("尚未启用受控的固定版本；原版 UA、在线引用与统计请求未完成整改，节点自查和流媒体解锁未知".into()), databases: &[] },
        ] }
    }

    #[cfg(test)]
    pub(super) fn check_place_fixture(origin: &str) -> Self {
        let mut registry = Self::default();
        registry
            .providers
            .retain(|provider| provider.id == "check-place");
        registry.providers[0].adapter = Some(Adapter::CheckPlace {
            origin: origin.into(),
        });
        registry
    }

    pub fn descriptions(&self) -> Vec<ProviderDescription> {
        self.providers
            .iter()
            .map(|provider| ProviderDescription {
                provider: provider.id.into(),
                label: provider.label.into(),
                kind: provider.kind.into(),
                execution: provider.execution.into(),
                enabled: provider.adapter.is_some(),
                reason: provider.reason.clone(),
                databases: provider
                    .databases
                    .iter()
                    .map(|(database, label)| DatabaseDescription {
                        database: (*database).into(),
                        label: (*label).into(),
                    })
                    .collect(),
            })
            .collect()
    }

    pub(super) fn enabled(&self) -> impl Iterator<Item = &Provider> {
        self.providers
            .iter()
            .filter(|provider| provider.adapter.is_some())
    }

    pub(super) fn mark_availability(&self, quality: &mut [IpQuality]) {
        for entry in quality {
            let provider = self
                .providers
                .iter()
                .find(|provider| provider.id == entry.provider);
            let available = provider.is_some_and(|provider| provider.adapter.is_some());
            let reason = provider
                .and_then(|provider| provider.reason.clone())
                .or_else(|| {
                    (!available).then(|| "此历史入口当前未注册或未启用，信息仅作历史参考".into())
                });
            for database in &mut entry.databases {
                database.available = Some(available);
                database.unavailable_reason = reason.clone();
                if !available && !database.fields.is_empty() {
                    database.historical = true;
                }
            }
        }
    }
}

impl Provider {
    pub(super) fn id(&self) -> &'static str {
        self.id
    }
    pub(super) fn databases(&self) -> &'static [(&'static str, &'static str)] {
        self.databases
    }

    pub(super) async fn query(
        &self,
        client: &Client,
        ip: &str,
        database: &str,
    ) -> Result<Vec<QualityField>, QueryError> {
        match self.adapter.as_ref() {
            Some(Adapter::CheckPlace { origin }) => {
                super::query_database(client, origin, ip, database).await
            }
            Some(Adapter::AbuseIpDb { endpoint, key }) => {
                let request = client
                    .get(endpoint)
                    .header("Key", key.clone())
                    .header(ACCEPT, "application/json")
                    .query(&[("ipAddress", ip), ("maxAgeInDays", "30")]);
                let value = super::query_json(request).await?;
                official_fields(&value, ip)
            }
            None => Err(QueryError::new(
                QueryErrorKind::NotAttempted,
                "此入口未启用，未向第三方查询",
            )),
        }
    }
}

fn official_fields(value: &Value, ip: &str) -> Result<Vec<QualityField>, QueryError> {
    let target = ip.parse::<IpAddr>().ok();
    let data = &value["data"];
    let valid = target.is_some()
        && super::fields::confirmed_response(value)
        && super::fields::confirmed_response(data)
        && data["ipAddress"]
            .as_str()
            .and_then(|ip| ip.parse::<IpAddr>().ok())
            == target
        && data["isPublic"] == Value::Bool(true)
        && data["ipVersion"].as_u64() == target.map(|ip| if ip.is_ipv4() { 4 } else { 6 });
    if !valid {
        return Err(QueryError::new(
            QueryErrorKind::SchemaMismatch,
            "正式接口未确认本次目标 IP、公网状态或 IP 版本，信息未知",
        ));
    }
    let fields = super::fields::parse_fields("abuseipdb-v2", value);
    if fields.is_empty() {
        return Err(QueryError::new(
            QueryErrorKind::SchemaMismatch,
            "正式接口没有可信的有效字段，信息未知",
        ));
    }
    Ok(fields)
}

#[cfg(test)]
mod tests;
