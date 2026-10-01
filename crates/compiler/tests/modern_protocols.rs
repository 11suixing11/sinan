#![forbid(unsafe_code)]

use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sinan_compiler::{
    Access, AcmeChallenge, Node, ProtocolConfig, SsMethod, TlsConfig, compile_client,
    compile_server, subscription_links,
};
use uuid::Uuid;

fn nodes() -> Vec<Node> {
    let tls = TlsConfig::Acme {
        email: "admin@example.com".into(),
        challenge: AcmeChallenge::Http01,
    };
    [
        ProtocolConfig::Hysteria2 { tls: tls.clone() },
        ProtocolConfig::Shadowsocks2022 {
            method: SsMethod::Aes128,
            password: STANDARD.encode([11; 16]),
        },
        ProtocolConfig::Shadowsocks2022 {
            method: SsMethod::Aes256,
            password: STANDARD.encode([12; 32]),
        },
        ProtocolConfig::Tuic { tls: tls.clone() },
        ProtocolConfig::Anytls { tls: tls.clone() },
        ProtocolConfig::Naive { tls },
        ProtocolConfig::SnellV6 {
            psk: STANDARD.encode([13; 32]),
        },
    ]
    .into_iter()
    .enumerate()
    .map(|(index, protocol_config)| Node {
        id: index as i64 + 1,
        name: format!("Test {index}"),
        port: 20100 + index as u16,
        public_host: "proxy.example.com".into(),
        sni: if protocol_config.tls().is_some() {
            "proxy.example.com".into()
        } else {
            String::new()
        },
        private_key: String::new(),
        public_key: String::new(),
        short_id: String::new(),
        users: [1, 2]
            .map(|user_id| Access {
                user_id,
                uuid: Uuid::from_u128(user_id as u128),
                credential: STANDARD.encode(vec![user_id as u8; protocol_config.credential_size()]),
            })
            .to_vec(),
        protocol_config,
    })
    .collect()
}

#[test]
fn all_protocols_preserve_identity_and_separate_client_secrets() {
    let nodes = nodes();
    let server: Value = serde_json::from_str(&compile_server(&nodes).unwrap()).unwrap();
    let client_text = compile_client(&nodes, 1).unwrap();
    let client: Value = serde_json::from_str(&client_text).unwrap();
    let inbounds = server["inbounds"].as_array().unwrap();
    assert_eq!(inbounds.len(), 7);
    assert_eq!(
        server["certificate_providers"][0]["domain"],
        json!(["proxy.example.com"])
    );
    assert_eq!(
        server["certificate_providers"][0]["data_directory"],
        "certificates"
    );
    assert_eq!(
        server["experimental"]["v2ray_api"]["stats"]["users"]
            .as_array()
            .unwrap()
            .len(),
        14
    );
    for (node, inbound) in nodes.iter().zip(inbounds) {
        let field = if node.protocol_config.kind() == "naive" {
            "username"
        } else {
            "name"
        };
        assert_eq!(inbound["users"][0][field], format!("u1_n{}", node.id));
        assert!(!client_text.contains(&node.users[1].credential));
        assert!(!client_text.contains(&node.users[1].uuid.to_string()));
        assert!(!client_text.contains("managed-tls"));
        assert!(!client_text.contains("admin@example.com"));
    }
    assert_eq!(
        client["outbounds"][2]["password"],
        format!("{}:{}", STANDARD.encode([11; 16]), STANDARD.encode([1; 16]))
    );
    assert_eq!(client["outbounds"][7]["version"], 6);
    assert_eq!(
        client["outbounds"][7]["userkey"],
        nodes[6].users[0].credential
    );
    assert!(
        subscription_links(&nodes, 1)
            .unwrap_err()
            .to_string()
            .contains("JSON")
    );
}

#[test]
fn deterministic_and_empty_nodes_do_not_request_certificates() {
    let mut reversed = nodes();
    reversed.reverse();
    for node in &mut reversed {
        node.users.reverse();
    }
    assert_eq!(
        compile_server(&nodes()).unwrap(),
        compile_server(&reversed).unwrap()
    );
    assert_eq!(
        compile_client(&nodes(), 1).unwrap(),
        compile_client(&reversed, 1).unwrap()
    );
    for node in &mut reversed {
        node.users.clear();
    }
    assert_eq!(
        compile_server(&reversed).unwrap(),
        compile_server(&[]).unwrap()
    );
}

#[test]
fn invalid_secrets_and_conflicting_acme_settings_are_rejected() {
    let mut model = nodes();
    model[0].users[1].credential = model[0].users[0].credential.clone();
    assert!(compile_server(&model).is_err());
    let mut model = nodes();
    model[1].users[0].credential = STANDARD.encode([0; 32]);
    assert!(compile_server(&model).is_err());
    let mut model = nodes();
    model[3].protocol_config = ProtocolConfig::Tuic {
        tls: TlsConfig::Acme {
            email: "other@example.com".into(),
            challenge: AcmeChallenge::Http01,
        },
    };
    assert!(compile_server(&model).is_err());
    let mut model = nodes();
    model[1].port = 80;
    assert!(compile_server(&model).is_err());
    model[1].port = 443;
    assert!(compile_server(&model).is_ok());
    // A UDP listener can share the HTTP challenge's numeric port.
    model[0].port = 80;
    assert!(compile_server(&model).is_ok());
}

#[test]
fn manual_certificate_private_key_never_enters_client_config() {
    let mut model = nodes();
    model[0].protocol_config = ProtocolConfig::Hysteria2 { tls: TlsConfig::Manual {
        certificate: "-----BEGIN CERTIFICATE-----\nsynthetic-test-only\n-----END CERTIFICATE-----".into(),
        key: "-----BEGIN PRIVATE KEY-----\nsynthetic-private-test-only\n-----END PRIVATE KEY-----".into(),
    }};
    let server = compile_server(&model).unwrap();
    let client = compile_client(&model, 1).unwrap();
    assert!(server.contains("synthetic-private-test-only"));
    assert!(!client.contains("synthetic-private-test-only"));
    assert!(client.contains("synthetic-test-only"));
    assert!(!client.contains("insecure"));
}

#[test]
#[ignore = "requires upstream v1.14.2 with QUIC, ACME, Naive and V2Ray API; set SINAN_TEST_SINGBOX"]
fn upstream_accepts_every_protocol_and_acme_provider() {
    let binary = std::env::var("SINAN_TEST_SINGBOX").unwrap();
    let directory = std::env::temp_dir().join(format!("sinan-modern-check-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&directory).unwrap();
    for (name, content) in [
        ("server", compile_server(&nodes()).unwrap()),
        ("client", compile_client(&nodes(), 1).unwrap()),
    ] {
        let path = directory.join(format!("{name}.json"));
        std::fs::write(&path, content).unwrap();
        let output = std::process::Command::new(&binary)
            .args(["check", "-c"])
            .arg(path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    std::fs::remove_dir_all(directory).unwrap();
}
