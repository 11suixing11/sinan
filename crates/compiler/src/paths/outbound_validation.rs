use super::transport_validation::{common, credential, duration, headers, optional_duration, text};
use crate::external::{HysteriaObfs, NormalizedOutbound};
use base64::{Engine, engine::general_purpose::STANDARD};
use std::collections::BTreeMap;
use uuid::Uuid;

struct Budget {
    remaining: usize,
}
impl std::io::Write for Budget {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.remaining {
            return Err(std::io::Error::other("configuration budget exceeded"));
        }
        self.remaining -= bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn auth(username: &Option<String>, password: &Option<String>) -> bool {
    username
        .as_ref()
        .is_none_or(|value| value.len() <= 65_536 && !value.contains('\0'))
        && password
            .as_ref()
            .is_none_or(|value| value.len() <= 65_536 && !value.contains('\0'))
        && (password.as_ref().is_none_or(|value| value.is_empty())
            || username.as_ref().is_some_and(|value| !value.is_empty()))
}
fn port_range(value: &str) -> bool {
    value.len() <= 65_536 && !value.is_empty() && value.split(',').all(|range| {
        if let Some((min, max)) = range.split_once(':') {
            matches!((min.parse::<u16>(), max.parse::<u16>()), (Ok(min), Ok(max)) if min > 0 && min <= max)
        } else { range.parse::<u16>().is_ok_and(|port| port > 0) }
    })
}
fn plugin_fields(value: &str) -> Option<BTreeMap<String, String>> {
    if !text(value) {
        return None;
    }
    let mut fields = BTreeMap::new();
    let mut chunks = vec![String::new()];
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            chunks.last_mut()?.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == ';' {
            if chunks.len() >= 32 {
                return None;
            }
            chunks.push(String::new());
        } else {
            chunks.last_mut()?.push(character);
        }
    }
    if escaped {
        return None;
    }
    for chunk in chunks {
        if chunk.is_empty() {
            continue;
        }
        let (key, value) = chunk.split_once('=').unwrap_or((&chunk, ""));
        if key.is_empty() || fields.insert(key.to_owned(), value.to_owned()).is_some() {
            return None;
        }
    }
    Some(fields)
}
fn plugin(name: &Option<String>, options: &Option<String>) -> bool {
    let Some(name) = name.as_deref() else {
        return options.is_none();
    };
    let Some(fields) = plugin_fields(options.as_deref().unwrap_or_default()) else {
        return false;
    };
    match name {
        "obfs-local" => {
            fields
                .keys()
                .all(|key| ["obfs", "obfs-host"].contains(&key.as_str()))
                && fields
                    .get("obfs")
                    .is_none_or(|value| ["http", "tls"].contains(&value.as_str()))
                && fields
                    .get("obfs-host")
                    .is_none_or(|value| value.is_empty() || crate::valid_public_host(value))
        }
        "v2ray-plugin" => {
            fields
                .keys()
                .all(|key| ["mode", "tls", "host", "path", "mux"].contains(&key.as_str()))
                && fields
                    .get("mode")
                    .is_none_or(|value| ["websocket", "quic"].contains(&value.as_str()))
                && fields.get("tls").is_none_or(|value| value.is_empty())
                && fields
                    .get("host")
                    .is_none_or(|value| crate::valid_public_host(value))
                && fields.get("path").is_none_or(|value| text(value))
                && fields
                    .get("mux")
                    .is_none_or(|value| value.parse::<u16>().is_ok())
        }
        _ => false,
    }
}
pub(super) fn plugin_quic(outbound: &NormalizedOutbound) -> bool {
    matches!(outbound, NormalizedOutbound::Shadowsocks { plugin: Some(name), plugin_opts, .. }
        if name == "v2ray-plugin" && plugin_fields(plugin_opts.as_deref().unwrap_or_default()).is_some_and(|fields| fields.get("mode").is_some_and(|mode| mode == "quic")))
}

pub(super) fn external_outbound(outbound: &NormalizedOutbound) -> Result<(), &'static str> {
    common(outbound.common())?;
    serde_json::to_writer(
        &mut Budget {
            remaining: 2 * 1024 * 1024,
        },
        outbound,
    )
    .map_err(|_| "outbound exceeds its bounded configuration budget")?;
    let common = outbound.common();
    let tls_enabled = common.tls.as_ref().is_some_and(|tls| tls.enabled);
    let reality = common
        .tls
        .as_ref()
        .is_some_and(|tls| tls.reality.as_ref().is_some_and(|reality| reality.enabled));
    let utls = common
        .tls
        .as_ref()
        .is_some_and(|tls| tls.utls.as_ref().is_some_and(|utls| utls.enabled));
    if reality && !matches!(outbound, NormalizedOutbound::Vless { .. }) {
        return Err("Reality is supported only on VLESS hops");
    }
    if common.transport.is_some()
        && !matches!(
            outbound,
            NormalizedOutbound::Vmess { .. }
                | NormalizedOutbound::Trojan { .. }
                | NormalizedOutbound::Vless { .. }
        )
    {
        return Err("this protocol does not preserve a V2Ray transport");
    }
    if common.multiplex.is_some()
        && !matches!(
            outbound,
            NormalizedOutbound::Shadowsocks { .. }
                | NormalizedOutbound::Vmess { .. }
                | NormalizedOutbound::Trojan { .. }
                | NormalizedOutbound::Vless { .. }
        )
    {
        return Err("this protocol does not preserve multiplex options");
    }
    if common.udp_over_tcp.is_some()
        && !matches!(
            outbound,
            NormalizedOutbound::Shadowsocks { .. } | NormalizedOutbound::Socks { .. }
        )
    {
        return Err("this protocol does not preserve UDP-over-TCP options");
    }
    if common.tls.is_some()
        && matches!(
            outbound,
            NormalizedOutbound::Shadowsocks { .. } | NormalizedOutbound::Socks { .. }
        )
    {
        return Err("this protocol does not preserve TLS options");
    }
    let valid = match outbound {
        NormalizedOutbound::Shadowsocks {
            method,
            password,
            plugin: name,
            plugin_opts,
            ..
        } => {
            let valid_method = [
                "none",
                "aes-128-gcm",
                "aes-192-gcm",
                "aes-256-gcm",
                "chacha20-ietf-poly1305",
                "xchacha20-ietf-poly1305",
                "2022-blake3-aes-128-gcm",
                "2022-blake3-aes-256-gcm",
                "2022-blake3-chacha20-poly1305",
                "aes-128-ctr",
                "aes-192-ctr",
                "aes-256-ctr",
                "aes-128-cfb",
                "aes-192-cfb",
                "aes-256-cfb",
                "rc4-md5",
                "chacha20-ietf",
                "xchacha20",
            ]
            .contains(&method.as_str());
            let valid_password = if method.starts_with("2022-") {
                let size = if method == "2022-blake3-aes-128-gcm" {
                    16
                } else {
                    32
                };
                password.len() <= 65_536
                    && password
                        .split(':')
                        .all(|part| STANDARD.decode(part).is_ok_and(|key| key.len() == size))
                    && (method != "2022-blake3-chacha20-poly1305" || !password.contains(':'))
            } else {
                (method == "none" && password.len() <= 65_536 && !password.contains('\0'))
                    || credential(password)
            };
            valid_method && valid_password && plugin(name, plugin_opts)
        }
        NormalizedOutbound::Vmess {
            uuid,
            security,
            packet_encoding,
            ..
        } => {
            Uuid::parse_str(uuid).is_ok()
                && [
                    "auto",
                    "none",
                    "zero",
                    "aes-128-cfb",
                    "aes-128-gcm",
                    "chacha20-poly1305",
                ]
                .contains(&security.as_str())
                && packet_encoding
                    .as_ref()
                    .is_none_or(|value| ["", "packetaddr", "xudp"].contains(&value.as_str()))
        }
        NormalizedOutbound::Trojan { password, .. } => credential(password),
        NormalizedOutbound::Vless {
            uuid,
            flow,
            packet_encoding,
            ..
        } => {
            Uuid::parse_str(uuid).is_ok()
                && ["", "xtls-rprx-vision"].contains(&flow.as_str())
                && ["", "packetaddr", "xudp"].contains(&packet_encoding.as_str())
                && (flow.is_empty() || (tls_enabled && common.transport.is_none()))
        }
        NormalizedOutbound::Hysteria2 {
            password,
            server_ports,
            hop_interval,
            hop_interval_max,
            up_mbps,
            down_mbps,
            obfs,
            bbr_profile,
            ..
        } => {
            let obfs = match obfs {
                None => true,
                Some(HysteriaObfs::Salamander { password }) => credential(password),
                Some(HysteriaObfs::Gecko {
                    password,
                    min_packet_size,
                    max_packet_size,
                }) => {
                    credential(password)
                        && (*max_packet_size == 0 || min_packet_size <= max_packet_size)
                }
            };
            tls_enabled
                && !utls
                && !reality
                && credential(password)
                && server_ports.len() <= 128
                && server_ports.iter().all(|ports| port_range(ports))
                && optional_duration(hop_interval)
                && optional_duration(hop_interval_max)
                && up_mbps.is_none_or(|value| (1..=1_000_000).contains(&value))
                && down_mbps.is_none_or(|value| (1..=1_000_000).contains(&value))
                && obfs
                && ["standard", "conservative", "aggressive"].contains(&bbr_profile.as_str())
        }
        NormalizedOutbound::Tuic {
            uuid,
            password,
            congestion_control,
            udp_relay_mode,
            udp_over_stream,
            heartbeat,
            ..
        } => {
            tls_enabled
                && !utls
                && !reality
                && Uuid::parse_str(uuid).is_ok()
                && credential(password)
                && ["cubic", "new_reno", "bbr"].contains(&congestion_control.as_str())
                && ["", "native", "quic"].contains(&udp_relay_mode.as_str())
                && (!udp_over_stream || udp_relay_mode.is_empty())
                && duration(heartbeat)
        }
        NormalizedOutbound::Anytls {
            password,
            idle_session_check_interval,
            idle_session_timeout,
            min_idle_session,
            client_metadata,
            ..
        } => {
            tls_enabled
                && !reality
                && credential(password)
                && duration(idle_session_check_interval)
                && duration(idle_session_timeout)
                && *min_idle_session <= 1024
                && client_metadata.as_ref().is_none_or(|value| text(value))
                && common.network.is_empty()
                && common.tcp_fast_open != Some(true)
        }
        NormalizedOutbound::Socks {
            version,
            username,
            password,
            ..
        } => {
            ["4", "4a", "5"].contains(&version.as_str())
                && auth(username, password)
                && (version == "5"
                    || (password.as_ref().is_none_or(|value| value.is_empty())
                        && common
                            .udp_over_tcp
                            .as_ref()
                            .is_none_or(|value| !value.enabled)))
        }
        NormalizedOutbound::Http {
            username,
            password,
            path,
            headers: fields,
            ..
        } => common.network.is_empty() && auth(username, password) && text(path) && headers(fields),
    };
    if !valid {
        return Err("outbound protocol parameters are invalid or unsupported");
    }
    Ok(())
}
