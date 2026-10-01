#![forbid(unsafe_code)]

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::Value;
use sinan_compiler::{
    Access, Node, Relay, compile_client, compile_server, compile_server_with_relays,
};
use uuid::Uuid;

fn node(id: i64, users: Vec<Access>) -> Node {
    Node {
        id,
        name: format!("node-{id}"),
        port: 443,
        public_host: "proxy.example.com".into(),
        sni: "www.example.com".into(),
        private_key: URL_SAFE_NO_PAD.encode([1; 32]),
        public_key: URL_SAFE_NO_PAD.encode([2; 32]),
        short_id: "1234abcd".into(),
        users,
        protocol_config: Default::default(),
    }
}
fn relay(id: i64) -> Relay {
    Relay {
        chain_id: id,
        entry_node_id: 1,
        exit_node_id: 2,
        uuid: Uuid::from_u128(11),
        public_host: "exit.example.com".into(),
        port: 443,
        sni: "www.example.com".into(),
        public_key: URL_SAFE_NO_PAD.encode([2; 32]),
        short_id: "1234abcd".into(),
    }
}
#[test]
fn routes_entry_and_counts_only_real_users() {
    let entry = node(
        1,
        vec![Access {
            credential: String::new(),
            user_id: 7,
            uuid: Uuid::from_u128(7),
        }],
    );
    let route = relay(1);
    let config: Value = serde_json::from_str(
        &compile_server_with_relays(std::slice::from_ref(&entry), std::slice::from_ref(&route))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(config["route"]["rules"][0]["outbound"], "chain-1");
    assert_eq!(config["outbounds"][1]["uuid"], route.uuid.to_string());
    assert_eq!(
        config["experimental"]["v2ray_api"]["stats"]["users"],
        serde_json::json!(["u7_n1"])
    );
    let exit: Value = serde_json::from_str(
        &compile_server_with_relays(&[node(2, vec![])], std::slice::from_ref(&route)).unwrap(),
    )
    .unwrap();
    assert_eq!(
        exit["inbounds"][0]["users"][0]["uuid"],
        route.uuid.to_string()
    );
    assert_eq!(
        exit["experimental"]["v2ray_api"]["stats"]["users"],
        serde_json::json!([])
    );
    let client = compile_client(&[entry], 7).unwrap();
    assert!(!client.contains(&route.uuid.to_string()));
    assert!(!client.contains("exit.example.com"));
}
#[test]
fn empty_relays_preserve_existing_bytes_and_invalid_topology_is_rejected() {
    let entry = node(1, vec![]);
    assert_eq!(
        compile_server(std::slice::from_ref(&entry)).unwrap(),
        compile_server_with_relays(std::slice::from_ref(&entry), &[]).unwrap()
    );
    let mut same_host_exit = node(2, vec![]);
    same_host_exit.port = 444;
    assert!(compile_server_with_relays(&[entry.clone(), same_host_exit], &[relay(1)]).is_err());
    assert!(compile_server_with_relays(&[entry], &[relay(1), relay(2)]).is_err());
}
#[test]
fn shared_exit_keeps_direct_stats_and_is_deterministic() {
    let exit = node(
        2,
        vec![Access {
            credential: String::new(),
            user_id: 9,
            uuid: Uuid::from_u128(9),
        }],
    );
    let first = relay(1);
    let mut second = relay(2);
    second.entry_node_id = 3;
    second.uuid = Uuid::from_u128(12);
    let a = compile_server_with_relays(
        std::slice::from_ref(&exit),
        &[first.clone(), second.clone()],
    )
    .unwrap();
    let b = compile_server_with_relays(&[exit], &[second, first]).unwrap();
    assert_eq!(a, b);
    let config: Value = serde_json::from_str(&a).unwrap();
    assert_eq!(config["inbounds"][0]["users"].as_array().unwrap().len(), 3);
    assert_eq!(
        config["experimental"]["v2ray_api"]["stats"]["users"],
        serde_json::json!(["u9_n2"])
    );
}

#[test]
#[ignore = "requires the pinned upstream binary in SINAN_GROUPS_RUNTIME"]
fn pinned_native_runtime_accepts_entry_exit_and_client_configs()
-> Result<(), Box<dyn std::error::Error>> {
    let runtime = std::env::var("SINAN_GROUPS_RUNTIME")?;
    let version = std::process::Command::new(&runtime)
        .arg("version")
        .output()?;
    assert!(version.status.success());
    assert!(String::from_utf8_lossy(&version.stdout).contains("sing-box version 1.14.2"));
    let directory = std::env::temp_dir().join(format!("sinan-relay-native-{}", Uuid::new_v4()));
    std::fs::create_dir(&directory)?;
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(directory.clone());
    let entry = node(
        1,
        vec![Access {
            credential: String::new(),
            user_id: 7,
            uuid: Uuid::from_u128(7),
        }],
    );
    let exit = node(2, vec![]);
    let relay = relay(1);
    for (name, config) in [
        (
            "entry",
            compile_server_with_relays(std::slice::from_ref(&entry), std::slice::from_ref(&relay))?,
        ),
        ("exit", compile_server_with_relays(&[exit], &[relay])?),
        ("client", compile_client(&[entry], 7)?),
    ] {
        let path = directory.join(format!("{name}.json"));
        std::fs::write(&path, config)?;
        let output = std::process::Command::new(&runtime)
            .arg("check")
            .arg("-c")
            .arg(&path)
            .output()?;
        assert!(
            output.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
