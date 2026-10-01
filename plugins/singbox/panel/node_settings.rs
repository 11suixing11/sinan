use crate::error::ApiResult;
use serde::{Deserialize, Deserializer};
use serde_json::{Value, json};
use sinan_compiler::{
    AnyTlsSettings, Hysteria2Settings, NodeSettings, RealitySettings, TuicSettings,
};

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsInput {
    pub listen: Option<String>,
    #[serde(default, deserialize_with = "nullable_port")]
    pub public_port: Option<Option<u16>>,
    pub tcp_fast_open: Option<bool>,
    pub tls_alpn: Option<Vec<String>>,
    pub reality: Option<RealitySettings>,
    pub hysteria2: Option<HysteriaInput>,
    pub tuic: Option<TuicSettings>,
    pub anytls: Option<AnyTlsSettings>,
}

fn nullable_port<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Option<u16>>, D::Error> {
    Option::<u16>::deserialize(deserializer).map(Some)
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HysteriaInput {
    pub up_mbps: Option<u32>,
    pub down_mbps: Option<u32>,
    pub ignore_client_bandwidth: bool,
    pub obfs_enabled: bool,
    pub obfs_password: Option<String>,
}

impl SettingsInput {
    pub fn build(self, previous: &Value) -> ApiResult<Value> {
        let mut settings: NodeSettings =
            serde_json::from_value(previous.clone()).map_err(anyhow::Error::from)?;
        if let Some(listen) = self.listen {
            settings.listen = listen;
        }
        if let Some(port) = self.public_port {
            settings.public_port = port;
        }
        if let Some(enabled) = self.tcp_fast_open {
            settings.tcp_fast_open = enabled;
        }
        if let Some(alpn) = self.tls_alpn {
            settings.tls_alpn = alpn;
        }
        if let Some(reality) = self.reality {
            settings.reality = reality;
        }
        if let Some(tuic) = self.tuic {
            settings.tuic = tuic;
        }
        if let Some(anytls) = self.anytls {
            settings.anytls = anytls;
        }
        if let Some(hy) = self.hysteria2 {
            let password = if hy.obfs_enabled {
                Some(
                    hy.obfs_password
                        .filter(|value| !value.is_empty())
                        .or(settings.hysteria2.obfs_password)
                        .unwrap_or_else(|| super::node_protocol::credential(32)),
                )
            } else {
                None
            };
            settings.hysteria2 = Hysteria2Settings {
                up_mbps: hy.up_mbps,
                down_mbps: hy.down_mbps,
                ignore_client_bandwidth: hy.ignore_client_bandwidth,
                obfs_password: password,
            };
        }
        Ok(serde_json::to_value(settings).map_err(anyhow::Error::from)?)
    }
}

pub(crate) fn view(value: Value) -> ApiResult<Value> {
    let settings: NodeSettings = serde_json::from_value(value).map_err(anyhow::Error::from)?;
    let configured = settings.hysteria2.obfs_password.is_some();
    let mut view = serde_json::to_value(settings).map_err(anyhow::Error::from)?;
    view["hysteria2"]
        .as_object_mut()
        .expect("settings object")
        .remove("obfs_password");
    view["hysteria2"]["obfs_enabled"] = json!(configured);
    Ok(view)
}
