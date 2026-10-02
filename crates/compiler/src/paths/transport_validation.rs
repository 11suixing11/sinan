use crate::external::{Common, Tls, Transport};
use base64::{Engine, engine::general_purpose::STANDARD};
use std::collections::BTreeMap;

pub(super) fn text(value: &str) -> bool {
    value.len() <= 65_536 && !value.chars().any(char::is_control)
}
pub(super) fn credential(value: &str) -> bool {
    !value.is_empty() && value.len() <= 65_536 && !value.contains('\0')
}
pub(super) fn duration(value: &str) -> bool {
    let units = [
        ("ns", 1_u64),
        ("us", 1_000),
        ("ms", 1_000_000),
        ("s", 1_000_000_000),
    ];
    units.into_iter().any(|(unit, factor)| {
        value.strip_suffix(unit).is_some_and(|number| {
            !number.is_empty()
                && number.bytes().all(|byte| byte.is_ascii_digit())
                && number
                    .parse::<u64>()
                    .ok()
                    .and_then(|number| number.checked_mul(factor))
                    .is_some_and(|nanoseconds| (1..=86_400_000_000_000).contains(&nanoseconds))
        })
    })
}
pub(super) fn optional_duration(value: &Option<String>) -> bool {
    value.as_deref().is_none_or(duration)
}
pub(super) fn headers(value: &BTreeMap<String, Vec<String>>) -> bool {
    value.len() <= 128
        && value.iter().all(|(key, values)| {
            !key.is_empty()
                && key.len() <= 256
                && key
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte))
                && values.len() <= 128
                && values.iter().all(|value| text(value))
        })
}

fn pem(values: &[String], label: &str) -> bool {
    values.len() <= 128
        && values.iter().all(|value| {
            if value.len() > 65_536 {
                return false;
            }
            let begin = format!("-----BEGIN {label}-----");
            let end = format!("-----END {label}-----");
            let mut rest = value.trim();
            let mut count = 0;
            while !rest.is_empty() {
                let Some(body) = rest.strip_prefix(&begin) else {
                    return false;
                };
                let Some((body, tail)) = body.split_once(&end) else {
                    return false;
                };
                // PEM whitespace is ignored only inside the exact delimited block.
                let encoded = body
                    .chars()
                    .filter(|character| !character.is_ascii_whitespace())
                    .collect::<String>();
                if !STANDARD
                    .decode(encoded)
                    .is_ok_and(|der| der.first() == Some(&0x30) && der.len() > 4)
                {
                    return false;
                }
                rest = tail.trim();
                count += 1;
                if count > 128 {
                    return false;
                }
            }
            count > 0
        })
}

fn tls(value: &Tls) -> Result<(), &'static str> {
    if value
        .server_name
        .as_ref()
        .is_some_and(|name| !crate::valid_public_host(name))
        || value.alpn.len() > 128
        || value
            .alpn
            .iter()
            .any(|value| value.is_empty() || value.len() > 255 || !text(value))
    {
        return Err("TLS server name or ALPN is invalid");
    }
    let versions = ["1.0", "1.1", "1.2", "1.3"];
    let min = value
        .min_version
        .as_deref()
        .map(|version| versions.iter().position(|allowed| *allowed == version));
    let max = value
        .max_version
        .as_deref()
        .map(|version| versions.iter().position(|allowed| *allowed == version));
    if min == Some(None)
        || max == Some(None)
        || matches!((min.flatten(), max.flatten()), (Some(min), Some(max)) if min > max)
    {
        return Err("TLS version range is invalid");
    }
    if !value.cipher_suites.is_empty() || value.ech.is_some() {
        return Err("TLS option requires an unsupported runtime capability");
    }
    if value.curve_preferences.len() > 5
        || value.curve_preferences.iter().any(|curve| {
            !["P256", "P384", "P521", "X25519", "X25519MLKEM768"].contains(&curve.as_str())
        })
    {
        return Err("TLS curve selection is invalid");
    }
    let utls = value.utls.as_ref().is_some_and(|utls| utls.enabled);
    let reality = value
        .reality
        .as_ref()
        .is_some_and(|reality| reality.enabled);
    if !value.enabled && (utls || reality) {
        return Err("TLS handshake extensions require TLS");
    }
    if utls && !value.curve_preferences.is_empty() {
        return Err("uTLS does not preserve custom curve preferences");
    }
    if let Some(utls) = &value.utls
        && ![
            "chrome_psk",
            "chrome_psk_shuffle",
            "chrome_padding_psk_shuffle",
            "chrome_pq",
            "chrome_pq_psk",
            "chrome",
            "firefox",
            "edge",
            "safari",
            "360",
            "qq",
            "ios",
            "android",
            "random",
            "randomized",
        ]
        .contains(&utls.fingerprint.as_str())
    {
        return Err("uTLS fingerprint is invalid");
    }
    if let Some(reality) = &value.reality
        && (!crate::valid_key(&reality.public_key)
            || reality.short_id.len() > 16
            || !reality.short_id.len().is_multiple_of(2)
            || !reality
                .short_id
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || (reality.enabled && (!utls || value.disable_sni == Some(true))))
    {
        return Err("Reality handshake configuration is invalid");
    }
    if value.certificate_public_key_sha256.len() > 128
        || value
            .certificate_public_key_sha256
            .iter()
            .any(|value| !STANDARD.decode(value).is_ok_and(|bytes| bytes.len() == 32))
        || (!value.certificate_public_key_sha256.is_empty() && !value.certificate.is_empty())
        || !pem(&value.certificate, "CERTIFICATE")
        || !pem(&value.client_certificate, "CERTIFICATE")
        || (value.client_certificate.is_empty() != value.client_key.is_empty())
    {
        return Err("TLS certificate or public-key pin configuration is invalid");
    }
    if value.client_key.len() > 1
        || value.client_key.iter().any(|value| {
            !["PRIVATE KEY", "RSA PRIVATE KEY", "EC PRIVATE KEY"]
                .iter()
                .any(|label| pem(std::slice::from_ref(value), label))
        })
    {
        return Err("TLS client key configuration is invalid");
    }
    Ok(())
}

pub(super) fn common(value: &Common) -> Result<(), &'static str> {
    if !crate::valid_public_host(&value.server) || value.server_port == 0 {
        return Err("outbound endpoint is invalid");
    }
    if value.network.len() > 2
        || value
            .network
            .iter()
            .any(|network| !["tcp", "udp"].contains(&network.as_str()))
        || (value.network.len() == 2 && value.network[0] == value.network[1])
        || !optional_duration(&value.connect_timeout)
    {
        return Err("outbound network or connection timeout is invalid");
    }
    if let Some(value) = &value.tls {
        tls(value)?;
    }
    if let Some(value) = &value.multiplex
        && (!["h2mux", "smux", "yamux"].contains(&value.protocol.as_str())
            || value.max_connections > 1024
            || value.min_streams > 4096
            || value.max_streams > 4096
            || (value.min_streams > 0
                && value.max_streams > 0
                && value.min_streams > value.max_streams))
    {
        return Err("multiplex configuration is invalid");
    }
    if value
        .udp_over_tcp
        .as_ref()
        .is_some_and(|value| ![1, 2].contains(&value.version))
    {
        return Err("UDP-over-TCP version is invalid");
    }
    if let Some(transport) = &value.transport {
        let valid = match transport {
            Transport::Ws {
                path,
                headers: fields,
                max_early_data,
                early_data_header_name,
            } => {
                text(path)
                    && headers(fields)
                    && *max_early_data <= 65_536
                    && text(early_data_header_name)
            }
            Transport::Http {
                host,
                path,
                method,
                headers: fields,
                idle_timeout,
                ping_timeout,
            } => {
                host.len() <= 128
                    && host.iter().all(|host| crate::valid_public_host(host))
                    && text(path)
                    && ["GET", "POST", "PUT", "PATCH", "HEAD", "OPTIONS", "DELETE"]
                        .contains(&method.as_str())
                    && headers(fields)
                    && optional_duration(idle_timeout)
                    && optional_duration(ping_timeout)
            }
            Transport::Grpc {
                service_name,
                idle_timeout,
                ping_timeout,
                ..
            } => {
                text(service_name)
                    && optional_duration(idle_timeout)
                    && optional_duration(ping_timeout)
            }
            Transport::Httpupgrade {
                host,
                path,
                headers: fields,
            } => {
                (host.is_empty() || crate::valid_public_host(host)) && text(path) && headers(fields)
            }
            Transport::Quic {} => value.tls.as_ref().is_some_and(|tls| tls.enabled),
        };
        if !valid {
            return Err("outbound transport configuration is invalid");
        }
    }
    Ok(())
}
