use super::parse::{MAX_BODY, MAX_DEPTH, MAX_NODES, MAX_SCALAR, digest, parse};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::json;

#[test]
fn four_subscription_formats_produce_equivalent_proxy_parameters() {
    let uri = format!(
        "ss://{}@proxy.example.com:443#测试节点",
        STANDARD.encode("aes-128-gcm:fixture-password")
    );
    let json = json!({"outbounds":[{"type":"shadowsocks","tag":"测试节点","server":"proxy.example.com","server_port":443,"method":"aes-128-gcm","password":"fixture-password"}]}).to_string();
    let yaml = "proxies:\n  - name: 测试节点\n    type: ss\n    server: proxy.example.com\n    port: 443\n    cipher: aes-128-gcm\n    password: fixture-password\n";
    let encoded = STANDARD.encode(&uri);
    let results = [&uri, &encoded, &json, yaml].map(|text| parse(text.as_bytes()).unwrap());
    for result in &results {
        assert!(result.rejected.is_empty());
        assert_eq!(result.nodes.len(), 1);
        assert_eq!(result.nodes[0].outbound, results[0].nodes[0].outbound);
        assert_eq!(
            result.nodes[0].identity_key,
            results[0].nodes[0].identity_key
        );
    }
}

#[test]
fn credentials_and_names_are_versions_not_identity() {
    let first = parse(b"proxies: [{name: first, type: ss, server: proxy.example.com, port: 443, cipher: aes-128-gcm, password: first-credential}]").unwrap();
    let second = parse(b"proxies: [{name: renamed, type: ss, server: proxy.example.com, port: 443, cipher: aes-128-gcm, password: rotated-credential}]").unwrap();
    assert_eq!(first.nodes[0].identity_key, second.nodes[0].identity_key);
    assert_ne!(first.nodes[0].outbound, second.nodes[0].outbound);
    let ambiguous = parse(b"proxies: [{name: first, type: ss, server: proxy.example.com, port: 443, cipher: aes-128-gcm, password: first-account}, {name: second, type: ss, server: proxy.example.com, port: 443, cipher: aes-128-gcm, password: second-account}]").unwrap();
    assert!(ambiguous.nodes.is_empty());
    assert_eq!(
        ambiguous.ambiguous_keys,
        vec![first.nodes[0].identity_key.clone()]
    );
    assert_eq!(ambiguous.rejected.len(), 2);
    let independent = parse(b"payload: [{provider_id: one, name: identical, type: http, server: proxy.example.com, port: 443}, {provider_id: two, name: identical, type: http, server: proxy.example.com, port: 443}]").unwrap();
    assert_eq!(independent.nodes.len(), 2);
    assert_ne!(
        independent.nodes[0].identity_key,
        independent.nodes[1].identity_key
    );
}

#[test]
fn renaming_and_reordering_keep_identity_and_semantic_digest() {
    let first = parse(b"proxies: [{name: first, type: http, server: PROXY.EXAMPLE.COM, port: 443, username: fixture, password: credential}, {name: second, type: socks5, server: proxy.example.com, port: 1080}]").unwrap();
    let reordered = parse(b"proxies: [{name: renamed-second, type: socks5, server: proxy.example.com, port: 1080}, {name: renamed-first, type: http, server: proxy.example.com, port: 443, username: fixture, password: credential}]").unwrap();
    for (original, renamed) in first.nodes.iter().zip(reordered.nodes.iter().rev()) {
        assert_ne!(original.name, renamed.name);
        assert_eq!(original.identity_key, renamed.identity_key);
        assert_eq!(original.outbound, renamed.outbound);
        assert_eq!(
            digest(&serde_json::to_vec(&original.outbound).unwrap()),
            digest(&serde_json::to_vec(&renamed.outbound).unwrap())
        );
    }
}

#[test]
fn rejects_duplicate_keys_depth_recursive_aliases_and_expansion() {
    assert!(parse(br#"{"outbounds":[],"outbounds":[]}"#).is_err());
    assert!(parse(b"proxies: []\nproxies: []").is_err());
    let deep = format!(
        "proxies: {}0{}",
        "[".repeat(MAX_DEPTH + 1),
        "]".repeat(MAX_DEPTH + 1)
    );
    assert_eq!(parse(deep.as_bytes()).err().unwrap().0, "structure_limit");
    assert_eq!(
        parse(b"proxies: &recursive [*recursive]").err().unwrap().0,
        "recursive_or_unknown_yaml_alias"
    );
    let mut bomb = String::from("a: &a [value, value, value, value, value, value, value, value]\n");
    let mut previous = "a".to_string();
    for i in 0..10 {
        let name = format!("b{i}");
        bomb.push_str(&format!(
            "{name}: &{name} [{}]\n",
            vec![format!("*{previous}"); 8].join(", ")
        ));
        previous = name;
    }
    bomb.push_str("proxies: []");
    assert_eq!(parse(bomb.as_bytes()).err().unwrap().0, "structure_limit");
    assert_eq!(
        parse(&vec![b'x'; MAX_BODY + 1]).err().unwrap().0,
        "body_limit"
    );
    let scalar = format!("ignored: '{}'\nproxies: []", "x".repeat(MAX_SCALAR + 1));
    assert_eq!(parse(scalar.as_bytes()).err().unwrap().0, "scalar_limit");
    let values = format!("ignored: [{}]\nproxies: []", "0,".repeat(100_001));
    assert_eq!(parse(values.as_bytes()).err().unwrap().0, "structure_limit");
    let json_depth = format!(
        "{{\"outbounds\":[],\"ignored\":{}0{}}}",
        "[".repeat(MAX_DEPTH + 1),
        "]".repeat(MAX_DEPTH + 1)
    );
    assert_eq!(
        parse(json_depth.as_bytes()).err().unwrap().0,
        "invalid_json_or_limit"
    );
    assert!(
        parse(
            "http://proxy.example.com:443\n"
                .repeat(MAX_NODES + 1)
                .as_bytes()
        )
        .is_err()
    );
}

#[test]
fn bounded_aliases_and_merges_work_without_accepting_duplicate_explicit_fields() {
    let value = parse(b"base: &base {type: http, server: proxy.example.com, port: 443}\nproxies:\n  - <<: *base\n    name: fixture\n").unwrap();
    assert_eq!(value.nodes.len(), 1);
    assert_eq!(value.nodes[0].outbound.server(), "proxy.example.com");
    assert!(
        parse(b"proxies: [{name: x, name: y, type: http, server: proxy.example.com, port: 443}]")
            .is_err()
    );
}

#[test]
fn reports_unsupported_semantics_without_exporting_secrets() {
    let value = parse(br#"{"outbounds":[{"type":"http","tag":"fixture","server":"proxy.example.com","server_port":443,"detour":"SECRET_TOKEN"}],"route":{"final":"do-not-import"}}"#).unwrap();
    assert!(value.nodes.is_empty());
    let public = serde_json::to_string(&value.rejected).unwrap();
    assert!(!public.contains("SECRET_TOKEN"));
    assert!(value.rejected[0].reason.contains("unsupported"));
    let value = parse(b"payload: [{name: fixture, type: ss, server: proxy.example.com, port: 443, cipher: aes-128-gcm, password: hidden, plugin: obfs, plugin-opts: {mode: tls}}]").unwrap();
    assert_eq!(value.rejected.len(), 1);
    assert!(
        parse(b"proxy-providers: {provider: {url: https://provider.example.com/private}} ")
            .is_err()
    );
}

#[test]
fn valid_proxies_never_import_source_routes_dns_providers_scripts_or_controls() {
    let json = br#"{
        "outbounds":[{"type":"http","tag":"source-tag","server":"proxy.example.com","server_port":443,"username":"fixture","password":"fixture-secret"}],
        "inbounds":[{"type":"mixed","listen":"0.0.0.0","listen_port":9999}],
        "route":{"final":"source-direct","rules":[{"action":"resolve","server":"source-dns"}]},
        "dns":{"servers":[{"type":"https","tag":"source-dns","server":"untrusted.example.com"}]},
        "experimental":{"clash_api":{"external_controller":"0.0.0.0:9998","secret":"source-control-secret"}},
        "script":"source-script-secret"
    }"#;
    let yaml = b"proxies:\n  - name: source-tag\n    type: http\n    server: proxy.example.com\n    port: 443\n    username: fixture\n    password: fixture-secret\nproxy-providers: {other: {type: http, url: https://provider.example.com/secret}}\nproxy-groups: [{name: source-group, type: select, proxies: [source-tag]}]\nrules: [MATCH,source-direct]\ndns: {enable: true, nameserver: [https://untrusted.example.com/dns-query]}\nscript: {code: source-script-secret}\nexternal-controller: 0.0.0.0:9998\nsecret: source-control-secret\n";
    let expected = json!({"type":"http","server":"proxy.example.com","server_port":443,"username":"fixture","password":"fixture-secret"});
    for bytes in [json.as_slice(), yaml.as_slice()] {
        let parsed = parse(bytes).unwrap();
        assert!(parsed.rejected.is_empty());
        assert_eq!(parsed.nodes.len(), 1);
        assert_eq!(parsed.nodes[0].outbound.0, expected);
        let rendered = parsed.nodes[0]
            .outbound
            .render(
                "compiled-tag",
                Some("compiled-previous"),
                Some("compiled-resolver"),
            )
            .unwrap();
        assert_eq!(rendered["tag"], "compiled-tag");
        assert_eq!(rendered["detour"], "compiled-previous");
        let output = rendered.to_string();
        for forbidden in [
            "source-tag",
            "source-direct",
            "source-dns",
            "source-control-secret",
            "source-script-secret",
            "provider.example.com",
            "untrusted.example.com",
            "0.0.0.0",
        ] {
            assert!(!output.contains(forbidden), "{forbidden}");
        }
    }
}

#[test]
fn sip002_distinguishes_ss2022_and_ipv6_percent_encoded_passwords() {
    let password = STANDARD.encode([1u8; 16]);
    let uri = format!("ss://2022-blake3-aes-128-gcm:{password}@%5B2001:db8::1%5D:443");
    let invalid = parse(uri.as_bytes()).unwrap();
    assert_eq!(invalid.rejected.len(), 1);
    let uri = format!("ss://2022-blake3-aes-128-gcm:{password}@[2001:db8::1]:443#fixture");
    let valid = parse(uri.as_bytes()).unwrap();
    assert_eq!(valid.nodes.len(), 1);
    assert_eq!(valid.nodes[0].outbound.server(), "2001:db8::1");
    let encoded = format!(
        "ss://{}@proxy.example.com:443",
        STANDARD.encode(format!("2022-blake3-aes-128-gcm:{password}"))
    );
    assert_eq!(
        parse(encoded.as_bytes()).unwrap().rejected[0].reason,
        "ss2022_requires_plain_userinfo"
    );
    let legacy = format!(
        "ss://{}",
        STANDARD.encode(format!(
            "2022-blake3-aes-128-gcm:{password}@proxy.example.com:443"
        ))
    );
    assert_eq!(
        parse(legacy.as_bytes()).unwrap().rejected[0].reason,
        "ss2022_requires_plain_userinfo"
    );
}
