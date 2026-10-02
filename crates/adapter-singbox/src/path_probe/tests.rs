use super::*;
use serde_json::json;
use std::{collections::BTreeMap, path::PathBuf};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

const ID: &str = "00000000-0000-0000-0000-000000000001";
const SECRET: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn spec(port: u16) -> RuntimeSpec {
    RuntimeSpec {
        revision: 1, kernel_version: "1.14.2".into(), config_hash: "a".repeat(64),
        binary_path: PathBuf::from("/fixture/sing-box"), revision_dir: PathBuf::from("/fixture/revision"),
        stats_listen: "127.0.0.1:18085".into(),
        files: BTreeMap::from([
            ("config.json".into(), json!({"outbounds":[{"type":"vless","tag":"path_1_2_0"}],
                "experimental":{"clash_api":{"external_controller":format!("127.0.0.1:{port}"),"secret":SECRET}}}).to_string()),
            (PLAN_FILE.into(), json!({"schema":1,"runtime_version":"1.14.2",
                "required_build_tags":["with_clash_api","with_v2ray_api"],
                "bindings":[{"id":ID,"selector":"path_1_2_0","target":"https://panel.example/health"}]}).to_string()),
        ]),
    }
}

fn change(spec: &mut RuntimeSpec, file: &str, edit: impl FnOnce(&mut serde_json::Value)) {
    let mut value: serde_json::Value = serde_json::from_str(&spec.files[file]).unwrap();
    edit(&mut value);
    spec.files.insert(file.into(), value.to_string());
}

#[test]
fn only_signed_bounded_concrete_bindings_can_use_the_private_controller() {
    assert!(configuration(&spec(18086)).unwrap().is_some());
    let mut legacy = spec(18086);
    legacy.files.remove(PLAN_FILE);
    assert!(configuration(&legacy).unwrap().is_none());
    for invalid in [
        "http://panel.example/health",
        "https://panel.example/",
        "https://panel.example/health?token=secret",
        "https://panel.example/health#fragment",
        "https://u:p@panel.example/health",
        "https://@panel.example/health",
        "https://panel.example:0/health",
        "https://panel.example\\other/health",
        " https://panel.example/health",
    ] {
        let mut value = spec(18086);
        change(&mut value, PLAN_FILE, |plan| {
            plan["bindings"][0]["target"] = json!(invalid)
        });
        assert!(configuration(&value).is_err(), "accepted {invalid}");
    }
    for controller in [
        "0.0.0.0:18086",
        "[::1]:18086",
        "127.0.0.1:0",
        "127.0.0.1:18085",
    ] {
        let mut value = spec(18086);
        change(&mut value, "config.json", |native| {
            native["experimental"]["clash_api"]["external_controller"] = json!(controller)
        });
        assert!(configuration(&value).is_err());
    }
    for kind in ["direct", "block", "selector", "urltest"] {
        let mut value = spec(18086);
        change(&mut value, "config.json", |native| {
            native["outbounds"][0]["type"] = json!(kind)
        });
        assert!(configuration(&value).is_err());
    }
    let mut duplicate = spec(18086);
    change(&mut duplicate, PLAN_FILE, |plan| {
        let binding = plan["bindings"][0].clone();
        plan["bindings"].as_array_mut().unwrap().push(binding);
    });
    assert!(configuration(&duplicate).is_err());
    let mut arbitrary = spec(18086);
    change(&mut arbitrary, PLAN_FILE, |plan| {
        plan["bindings"][0]["selector"] = json!("path/../../config")
    });
    assert!(configuration(&arbitrary).is_err());
    let mut extra = spec(18086);
    change(&mut extra, PLAN_FILE, |plan| {
        plan["command"] = json!("arbitrary")
    });
    assert!(configuration(&extra).is_err());
    let mut oversized = spec(18086);
    oversized
        .files
        .insert(PLAN_FILE.into(), " ".repeat(MAX_PLAN + 1));
    assert!(configuration(&oversized).is_err());
}

#[test]
fn fixed_runtime_and_actual_complete_build_features_are_required() {
    let value = spec(18086);
    assert!(
        validate_build(
            &value,
            "sing-box version 1.14.2\nTags: with_clash_api,with_v2ray_api,with_utls\n"
        )
        .is_ok()
    );
    for output in [
        "Tags: with_v2ray_api\n",
        "Tags: with_clash_api2,with_v2ray_api\n",
        "Tags: with_clash_api\n",
        "sing-box version 1.14.2\n",
    ] {
        assert!(validate_build(&value, output).is_err());
    }
    let mut another = value.clone();
    another.kernel_version = "1.14.3".into();
    assert!(configuration(&another).is_err());
    let mut tls = value;
    change(&mut tls, PLAN_FILE, |plan| {
        plan["required_build_tags"]
            .as_array_mut()
            .unwrap()
            .push(json!("with_utls"))
    });
    assert!(validate_build(&tls, "Tags: with_clash_api,with_v2ray_api\n").is_err());
    assert!(validate_build(&tls, "Tags: with_clash_api,with_v2ray_api,with_utls\n").is_ok());
}

async fn controller(response: String) -> (Prepared, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = Vec::new();
        loop {
            let mut buffer = [0; 1024];
            let read = socket.read(&mut buffer).await.unwrap();
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..read]);
            if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
            assert!(bytes.len() <= 16384);
        }
        socket.write_all(response.as_bytes()).await.unwrap();
        drop(socket);
        assert!(
            tokio::time::timeout(Duration::from_millis(100), listener.accept())
                .await
                .is_err(),
            "unexpected retry or redirect"
        );
        String::from_utf8(bytes).unwrap()
    });
    (
        Prepared {
            spec: spec(port),
            listen_ports: Vec::new(),
        },
        task,
    )
}

fn response(status: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

#[tokio::test]
async fn request_uses_authenticated_loopback_and_only_the_signed_selector_and_target() {
    let (runtime, request) = controller(response("200 OK", r#"{"delay":17}"#)).await;
    assert_eq!(
        execute(&runtime, ID).await.unwrap(),
        RuntimeProbeMeasurement { elapsed_ms: 17 }
    );
    let request = request.await.unwrap();
    assert!(request.starts_with("GET /proxies/path_1_2_0/delay?url=https%3A%2F%2Fpanel.example%2Fhealth&timeout=4500 HTTP/1.1\r\n"));
    assert!(
        request
            .to_ascii_lowercase()
            .contains(&format!("authorization: bearer {SECRET}"))
    );
}

#[tokio::test]
async fn failures_are_bounded_redacted_and_never_retried_or_redirected() {
    let redirect = "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:9/secret\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned();
    for wire in [
        redirect,
        response("503 Unavailable", SECRET),
        response("200 OK", "not JSON"),
        response("200 OK", r#"{"delay":0}"#),
        response("200 OK", r#"{"delay":4501}"#),
        response("200 OK", r#"{"delay":2,"secret":"sensitive"}"#),
        response("200 OK", &"x".repeat(4097)),
    ] {
        let (runtime, request) = controller(wire).await;
        let error = execute(&runtime, ID).await.unwrap_err().to_string();
        assert!(
            !error.contains(SECRET)
                && !error.contains("panel.example")
                && !error.contains("sensitive")
        );
        request.await.unwrap();
    }
    let runtime = Prepared {
        spec: spec(9),
        listen_ports: Vec::new(),
    };
    assert!(
        execute(&runtime, "00000000-0000-0000-0000-000000000099")
            .await
            .unwrap_err()
            .to_string()
            .contains("absent")
    );
}

#[tokio::test]
async fn controller_cannot_stream_past_budget_without_content_length() {
    let body = "x".repeat(4097);
    let wire = format!(
        "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{:x}\r\n{body}\r\n0\r\n\r\n",
        body.len()
    );
    let (runtime, request) = controller(wire).await;
    assert!(
        execute(&runtime, ID)
            .await
            .unwrap_err()
            .to_string()
            .contains("budget")
    );
    request.await.unwrap();
}
