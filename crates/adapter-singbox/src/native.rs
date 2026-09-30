use anyhow::{Context, Result, bail};
use sinan_adapter_sdk::RuntimeSpec;
use std::{
    collections::BTreeSet,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
};

pub(crate) fn stats_address(address: &str) -> Result<SocketAddr> {
    let address: SocketAddr = address
        .parse()
        .context("invalid statistics listen address")?;
    if address.ip() != IpAddr::V4(Ipv4Addr::LOCALHOST) || address.port() == 0 {
        bail!("statistics API must listen on 127.0.0.1 with a nonzero port");
    }
    Ok(address)
}

pub(crate) fn listen_addresses(spec: &RuntimeSpec) -> Result<Vec<SocketAddr>> {
    let stats = stats_address(&spec.stats_listen)?;
    if spec.files.len() != 1 {
        bail!("runtime bundle must contain only config.json");
    }
    let config: serde_json::Value = serde_json::from_str(
        spec.files
            .get("config.json")
            .context("missing config.json")?,
    )?;
    let api = &config["experimental"]["v2ray_api"];
    if api["listen"].as_str() != Some(spec.stats_listen.as_str())
        || api["stats"]["enabled"].as_bool() != Some(true)
    {
        bail!("native statistics configuration must match the runtime specification");
    }
    let mut ports = BTreeSet::new();
    let mut addresses = Vec::new();
    for inbound in config["inbounds"]
        .as_array()
        .context("inbounds must be an array")?
    {
        if inbound["type"].as_str() != Some("vless") {
            bail!("unsupported native inbound type");
        }
        let port: u16 = inbound["listen_port"]
            .as_u64()
            .context("inbound has no listen port")?
            .try_into()?;
        if port == 0 || port == stats.port() || !ports.insert(port) {
            bail!("invalid or duplicate native listen port");
        }
        let listen: IpAddr = inbound["listen"]
            .as_str()
            .context("inbound has no listen address")?
            .parse()?;
        let probe = match listen {
            IpAddr::V4(ip) if ip.is_unspecified() => IpAddr::V4(Ipv4Addr::LOCALHOST),
            IpAddr::V6(ip) if ip.is_unspecified() => IpAddr::V6(Ipv6Addr::LOCALHOST),
            address => address,
        };
        addresses.push(SocketAddr::new(probe, port));
    }
    addresses.sort();
    Ok(addresses)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcard_ports_are_probed_on_matching_loopback_family() {
        let spec = RuntimeSpec {
            revision: 1,
            kernel_version: "1.14.2".into(),
            config_hash: String::new(),
            binary_path: "/tmp/test-runtime".into(),
            revision_dir: "/tmp/test-revision".into(),
            stats_listen: "127.0.0.1:18085".into(),
            files: [("config.json".into(), serde_json::json!({
                "inbounds":[
                    {"type":"vless","listen":"::","listen_port":20001},
                    {"type":"vless","listen":"0.0.0.0","listen_port":20002}
                ],
                "experimental":{"v2ray_api":{"listen":"127.0.0.1:18085", "stats":{"enabled":true}}}
            }).to_string())].into(),
        };
        let addresses = listen_addresses(&spec).unwrap();
        assert!(addresses.contains(&"[::1]:20001".parse().unwrap()));
        assert!(addresses.contains(&"127.0.0.1:20002".parse().unwrap()));
    }
}
