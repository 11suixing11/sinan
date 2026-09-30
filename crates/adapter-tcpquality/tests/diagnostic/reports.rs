use super::*;

#[tokio::test]
async fn partial_report_is_readable_after_new_adapter_and_remains_unknown_without_samples() {
    let fixture = Fixture::new(2);
    let spec = fixture.spec();
    fixture.workspace();
    let body = report(&spec, &fixture.scope);
    fixture.write("targets.json", spec.options["targets"].as_bytes());
    fixture.write("result.json", body.to_string().as_bytes());
    fixture.write(
        "sections/tcp_summary.json",
        &chapter("tcp_summary", &body, false),
    );
    std::fs::remove_file(&spec.binary_path).unwrap();
    for _ in 0..2 {
        let saved = TcpQualityAdapter::new()
            .collect(&spec)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(saved.report_url, None);
        let value: Value = serde_json::from_str(&saved.text).unwrap();
        assert_eq!(value, body);
        let sections = TcpQualityAdapter::new()
            .collect_sections(&spec)
            .await
            .unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].name, "tcp_summary");
        assert!(!sections[0].complete);
        assert_eq!(value["complete"], false);
        assert_eq!(
            value["targets"][0]["summary"]["latency_mean_ms"],
            Value::Null
        );
        assert_eq!(
            value["targets"][0]["summary"]["connection_success_percent"],
            Value::Null
        );
    }
    assert_eq!(
        std::fs::read_to_string(spec.job_dir.join("result.json")).unwrap(),
        body.to_string()
    );
}

fn completed(spec: &DiagnosticSpec, scope: &Value, success: bool) -> Value {
    let mut body = report(spec, scope);
    let start = body["started_at_ms"].as_u64().unwrap();
    body["finished_at_ms"] = json!(start);
    body["complete"] = json!(true);
    for target in body["targets"].as_array_mut().unwrap() {
        target["address"] = json!("127.0.0.1:443");
        target["complete"] = json!(true);
        target["status"] = json!("completed");
        target["samples"] = json!(
            (0..4)
                .map(|_| json!({"attempted_at_ms":start,"elapsed_ms":0.0,
            "latency_ms":if success { json!(0.0) } else { Value::Null },
            "error":if success { Value::Null } else { json!("connect_refused") }}))
                .collect::<Vec<_>>()
        );
        target["summary"] = json!({"attempted":4,"succeeded":if success{4}else{0},
            "connection_success_percent":if success{100.0}else{0.0},
            "latency_min_ms":if success{json!(0.0)}else{Value::Null},
            "latency_mean_ms":if success{json!(0.0)}else{Value::Null},
            "latency_max_ms":if success{json!(0.0)}else{Value::Null}});
    }
    body
}
#[tokio::test]
async fn real_zero_and_failure_rate_are_distinct_from_unknown_and_invalid_default_latency() {
    let fixture = Fixture::new(1);
    let spec = fixture.spec();
    fixture.workspace();
    for success in [true, false] {
        let body = completed(&spec, &fixture.scope, success);
        fixture.write("result.json", body.to_string().as_bytes());
        assert!(
            TcpQualityAdapter::new()
                .collect(&spec)
                .await
                .unwrap()
                .is_some()
        );
        let mut bad = body;
        bad["targets"][0]["summary"]["latency_mean_ms"] =
            if success { Value::Null } else { json!(0.0) };
        fixture.write("result.json", bad.to_string().as_bytes());
        assert!(TcpQualityAdapter::new().collect(&spec).await.is_err());
    }
}

#[tokio::test]
async fn report_identity_parameters_timestamps_safety_and_statistics_must_match_frozen_scope() {
    let fixture = Fixture::new(1);
    let spec = fixture.spec();
    fixture.workspace();
    let original = completed(&spec, &fixture.scope, false);
    let mutations: Vec<(&str, Value)> = vec![
        ("/schema", json!(2)),
        ("/method", json!("speedtest")),
        ("/semantics", json!("干净")),
        ("/engine/version", json!("0.4.0")),
        ("/engine/source_commit", Value::Null),
        ("/target_digest", json!("0".repeat(64))),
        ("/parameters/ip_version", json!("6")),
        ("/parameters/count", json!(8)),
        ("/parameters/concurrency", json!(2)),
        ("/parameters/total_timeout_ms", json!(120000)),
        ("/parameters/dns_timeout_ms", json!(5000)),
        ("/upload_enabled", json!(true)),
        ("/ranking_enabled", json!(true)),
        ("/speedtest_enabled", json!(true)),
        ("/started_at_ms", json!(0)),
        ("/finished_at_ms", json!(u64::MAX)),
        ("/targets/0/address", json!("224.0.0.1:443")),
        ("/targets/0/target/region", json!("changed")),
        ("/targets/0/summary/attempted", json!(3)),
        ("/targets/0/summary/succeeded", json!(4)),
        (
            "/targets/0/summary/connection_success_percent",
            json!(100.0),
        ),
        ("/targets/0/samples/0/latency_ms", json!(0.0)),
        ("/targets/0/samples/0/error", json!("clean")),
        ("/targets/0/samples/0/attempted_at_ms", json!(0)),
    ];
    for (path, value) in mutations {
        let mut body = original.clone();
        *body.pointer_mut(path).unwrap() = value;
        fixture.write("result.json", body.to_string().as_bytes());
        assert!(
            TcpQualityAdapter::new().collect(&spec).await.is_err(),
            "{path}"
        );
    }
    for bytes in [
        b"null".to_vec(),
        b"<html>403</html>".to_vec(),
        vec![b'x'; 64 * 1024 + 1],
    ] {
        fixture.write("result.json", &bytes);
        assert!(TcpQualityAdapter::new().collect(&spec).await.is_err());
    }
}

#[tokio::test]
async fn bad_chapters_and_bad_main_report_do_not_hide_other_saved_chapters_or_expand_the_ten_name_scope()
 {
    let fixture = Fixture::new(8);
    let spec = fixture.spec();
    fixture.workspace();
    let body = report(&spec, &fixture.scope);
    fixture.write("result.json", b"bad whole report");
    fixture.write(
        "sections/tcp_scope.json",
        &chapter("tcp_scope", &body, true),
    );
    fixture.write(
        "sections/tcp_summary.json",
        &chapter("tcp_summary", &body, false),
    );
    for target in body["targets"].as_array().unwrap() {
        let name = format!(
            "tcp_target_{}",
            target["target"]["id"].as_str().unwrap().replace('-', "")
        );
        fixture.write(
            &format!("sections/{name}.json"),
            &chapter(&name, target, false),
        );
    }
    fixture.write(
        "sections/untrusted.json",
        &chapter("untrusted", &body, true),
    );
    let adapter = TcpQualityAdapter::new();
    assert_eq!(adapter.collect_sections(&spec).await.unwrap().len(), 10);
    let first = "tcp_target_10000000000040008000000000000000";
    fixture.write(
        &format!("sections/{first}.json"),
        &vec![b'x'; 64 * 1024 + 1],
    );
    fixture.write("sections/tcp_summary.json", b"{bad json");
    let chapters = adapter.collect_sections(&spec).await.unwrap();
    assert_eq!(chapters.len(), 8);
    assert!(chapters.iter().any(|chapter| chapter.name == "tcp_scope"));
    assert!(chapters.iter().all(|chapter| chapter.name != first
        && chapter.name != "tcp_summary"
        && chapter.name != "untrusted"));
    assert!(adapter.collect(&spec).await.is_err());
    assert_eq!(
        std::fs::read(spec.job_dir.join("result.json")).unwrap(),
        b"bad whole report"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn collection_rejects_public_linked_and_symbolic_files_and_preserves_good_partial_chapters() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let fixture = Fixture::new(1);
    let spec = fixture.spec();
    fixture.workspace();
    let body = report(&spec, &fixture.scope);
    fixture.write("result.json", body.to_string().as_bytes());
    let file = spec.job_dir.join("result.json");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(TcpQualityAdapter::new().collect(&spec).await.is_err());
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::hard_link(&file, spec.job_dir.join("linked.json")).unwrap();
    assert!(TcpQualityAdapter::new().collect(&spec).await.is_err());
    std::fs::remove_file(spec.job_dir.join("linked.json")).unwrap();
    std::fs::rename(&file, spec.job_dir.join("original.json")).unwrap();
    symlink(spec.job_dir.join("original.json"), &file).unwrap();
    assert!(TcpQualityAdapter::new().collect(&spec).await.is_err());
    fixture.write(
        "sections/tcp_scope.json",
        &chapter("tcp_scope", &body, true),
    );
    symlink(
        spec.job_dir.join("original.json"),
        spec.job_dir.join("sections/tcp_summary.json"),
    )
    .unwrap();
    let sections = TcpQualityAdapter::new()
        .collect_sections(&spec)
        .await
        .unwrap();
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].name, "tcp_scope");
}

#[tokio::test]
async fn completed_family_failure_stays_unknown_but_cancelled_targets_cannot_claim_completion() {
    let mut fixture = Fixture::new(1);
    fixture.scope["targets"][0]["target"] = json!("::1");
    let spec = fixture.spec();
    fixture.workspace();
    let mut body = report(&spec, &fixture.scope);
    body["complete"] = json!(true);
    body["finished_at_ms"] = body["started_at_ms"].clone();
    body["targets"][0]["complete"] = json!(true);
    body["targets"][0]["status"] = json!("failed");
    body["targets"][0]["error"] = json!("ip_family_unavailable");
    fixture.write("result.json", body.to_string().as_bytes());
    assert!(
        TcpQualityAdapter::new()
            .collect(&spec)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(
        body["targets"][0]["summary"]["connection_success_percent"],
        Value::Null
    );
    for error in [
        "total_timeout",
        "worker_failed",
        "utc_unavailable",
        "dns_timeout",
    ] {
        body["targets"][0]["error"] = json!(error);
        fixture.write("result.json", body.to_string().as_bytes());
        assert!(
            TcpQualityAdapter::new().collect(&spec).await.is_err(),
            "{error}"
        );
    }
}
