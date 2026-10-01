use base64::{
    Engine,
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD},
};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sinan_compiler::external::ExternalOutbound;
use std::collections::BTreeMap;

pub const MAX_BODY: usize = 2 * 1024 * 1024;
pub const MAX_NODES: usize = 5000;
pub const MAX_DEPTH: usize = 64;
pub const MAX_SCALAR: usize = 64 * 1024;
pub const PARSER_VERSION: &str = "sinan-subscriptions-1";

#[derive(Debug, Clone, thiserror::Error)]
#[error("{0}")]
pub struct ImportError(pub &'static str);

#[derive(Clone)]
pub struct ParsedNode {
    pub name: String,
    pub identity_key: String,
    pub outbound: ExternalOutbound,
}

#[derive(Clone, Serialize)]
pub struct RejectedNode {
    pub index: usize,
    pub name: String,
    pub reason: String,
}

pub struct ParsedBatch {
    pub format: &'static str,
    pub nodes: Vec<ParsedNode>,
    pub rejected: Vec<RejectedNode>,
    pub ambiguous_keys: Vec<String>,
}

pub fn digest(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}

pub fn parse(bytes: &[u8]) -> Result<ParsedBatch, ImportError> {
    if bytes.len() > MAX_BODY {
        return Err(ImportError("body_limit"));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| ImportError("invalid_utf8"))?
        .trim_start_matches('\u{feff}')
        .trim();
    if text.is_empty() {
        return Err(ImportError("empty_configuration"));
    }
    if text.starts_with('<') {
        return Err(ImportError("html_response"));
    }
    if text.starts_with('{') || text.starts_with('[') {
        return from_object(super::structured::json(text)?, "sing_box_json");
    }
    if text
        .lines()
        .map(str::trim)
        .find(|v| !v.is_empty() && !v.starts_with('#'))
        .is_some_and(|v| {
            v.split_once("://").is_some_and(|(scheme, _)| {
                !scheme.is_empty()
                    && scheme
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'+' | b'-' | b'.'))
            })
        })
    {
        return from_uris(text, "uri");
    }
    if !text.contains(':') {
        let decoded = decode_base64(text)?;
        let decoded = std::str::from_utf8(&decoded)
            .map_err(|_| ImportError("invalid_base64_configuration"))?;
        if !decoded.contains("://") {
            return Err(ImportError("invalid_base64_configuration"));
        }
        return from_uris(decoded, "base64_uri");
    }
    from_object(super::structured::yaml(text)?, "clash_yaml")
}

fn from_object(value: Value, format: &'static str) -> Result<ParsedBatch, ImportError> {
    let root = value
        .as_object()
        .ok_or(ImportError("configuration_must_be_object"))?;
    let (nodes, is_clash) = if let Some(outbounds) = root.get("outbounds") {
        if root.contains_key("proxies") || root.contains_key("payload") {
            return Err(ImportError("ambiguous_configuration_format"));
        }
        (outbounds, false)
    } else if let Some(proxies) = root.get("proxies").or_else(|| root.get("payload")) {
        if root.contains_key("proxies") && root.contains_key("payload") {
            return Err(ImportError("ambiguous_configuration_format"));
        }
        (proxies, true)
    } else {
        return Err(ImportError(if root.contains_key("proxy-providers") {
            "provider_url_without_nodes"
        } else {
            "missing_proxy_nodes"
        }));
    };
    let nodes = nodes.as_array().ok_or(ImportError("nodes_must_be_array"))?;
    if nodes.is_empty() || nodes.len() > MAX_NODES {
        return Err(ImportError("node_count_limit"));
    }
    let mut batch = ParsedBatch {
        format: if is_clash { "clash_yaml" } else { format },
        nodes: Vec::new(),
        rejected: Vec::new(),
        ambiguous_keys: Vec::new(),
    };
    for (index, value) in nodes.iter().enumerate() {
        let name = public_name(
            value
                .get(if is_clash { "name" } else { "tag" })
                .and_then(Value::as_str),
            index,
        );
        let result = if is_clash {
            super::mihomo::convert(value.clone())
        } else {
            sing_box(value.clone())
        };
        append(&mut batch, index, name, result);
    }
    identify(batch)
}

fn sing_box(mut value: Value) -> Result<(ExternalOutbound, Option<String>), ImportError> {
    let object = value
        .as_object_mut()
        .ok_or(ImportError("invalid_node_object"))?;
    let provider = provider_id(object.remove("provider_id"))?;
    object.remove("tag");
    // Dialer dependencies cannot be silently removed without changing node semantics.
    let outbound = ExternalOutbound(value);
    outbound
        .validate()
        .map_err(|_| ImportError("unsupported_or_invalid_proxy_parameter"))?;
    Ok((outbound, provider))
}

fn from_uris(text: &str, format: &'static str) -> Result<ParsedBatch, ImportError> {
    let mut batch = ParsedBatch {
        format,
        nodes: Vec::new(),
        rejected: Vec::new(),
        ambiguous_keys: Vec::new(),
    };
    for line in text
        .lines()
        .map(str::trim)
        .filter(|v| !v.is_empty() && !v.starts_with('#'))
    {
        let index = batch.nodes.len() + batch.rejected.len();
        if index >= MAX_NODES {
            return Err(ImportError("node_count_limit"));
        }
        if line.len() > MAX_SCALAR {
            return Err(ImportError("scalar_limit"));
        }
        let result = super::uri::parse(line);
        let name = result
            .as_ref()
            .map(|v| public_name(Some(&v.1), index))
            .unwrap_or_else(|_| public_name(None, index));
        append(&mut batch, index, name, result.map(|v| (v.0, None)));
    }
    if batch.nodes.is_empty() && batch.rejected.is_empty() {
        return Err(ImportError("empty_configuration"));
    }
    identify(batch)
}

fn append(
    batch: &mut ParsedBatch,
    index: usize,
    name: String,
    result: Result<(ExternalOutbound, Option<String>), ImportError>,
) {
    match result {
        Ok((mut outbound, provider)) => {
            let server = outbound
                .server()
                .parse::<std::net::IpAddr>()
                .map(|v| v.to_string())
                .unwrap_or_else(|_| outbound.server().to_ascii_lowercase());
            outbound.0["server"] = Value::String(server);
            if let Some(Value::String(sni)) = outbound.0.pointer_mut("/tls/server_name") {
                *sni = sni.to_ascii_lowercase();
            }
            let identity_key = match provider {
                Some(value) => format!("provider:{}", digest(value.as_bytes())),
                None => format!(
                    "endpoint:{}",
                    digest(&serde_json::to_vec(&outbound.identity_value()).expect("JSON value"))
                ),
            };
            batch.nodes.push(ParsedNode {
                name,
                identity_key,
                outbound,
            });
        }
        Err(error) => batch.rejected.push(RejectedNode {
            index,
            name,
            reason: error.0.into(),
        }),
    }
}

fn identify(mut batch: ParsedBatch) -> Result<ParsedBatch, ImportError> {
    let mut counts = BTreeMap::<String, usize>::new();
    for node in &batch.nodes {
        *counts.entry(node.identity_key.clone()).or_default() += 1;
    }
    batch.ambiguous_keys = counts
        .into_iter()
        .filter_map(|(key, count)| (count > 1).then_some(key))
        .collect();
    let mut nodes = Vec::new();
    for (index, node) in batch.nodes.into_iter().enumerate() {
        if batch
            .ambiguous_keys
            .binary_search(&node.identity_key)
            .is_ok()
        {
            batch.rejected.push(RejectedNode {
                index,
                name: node.name,
                reason: "ambiguous_node_identity".into(),
            });
        } else {
            nodes.push(node);
        }
    }
    batch.nodes = nodes;
    Ok(batch)
}

pub(super) fn provider_id(value: Option<Value>) -> Result<Option<String>, ImportError> {
    match value {
        None => Ok(None),
        Some(Value::String(value))
            if !value.trim().is_empty()
                && value.len() <= 256
                && !value.chars().any(char::is_control) =>
        {
            Ok(Some(value))
        }
        _ => Err(ImportError("invalid_provider_identity")),
    }
}

pub(super) fn public_name(name: Option<&str>, index: usize) -> String {
    let name = name.unwrap_or_default().trim();
    if name.is_empty() || name.contains("://") || name.chars().any(char::is_control) {
        format!("节点 {}", index + 1)
    } else {
        name.chars().take(160).collect()
    }
}

pub(super) fn decode_base64(value: &str) -> Result<Vec<u8>, ImportError> {
    let value: String = value.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    if value.len() > MAX_BODY {
        return Err(ImportError("body_limit"));
    }
    for engine in [&STANDARD, &STANDARD_NO_PAD, &URL_SAFE, &URL_SAFE_NO_PAD] {
        if let Ok(bytes) = engine.decode(&value)
            && bytes.len() <= MAX_BODY
        {
            return Ok(bytes);
        }
    }
    Err(ImportError("invalid_base64_configuration"))
}
