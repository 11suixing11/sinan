#![forbid(unsafe_code)]

use super::*;

const R2: &str = "a92fca6c0067df29ddd03fdc2fee6f3000f64545-r2";
const R3: &str = "a92fca6c0067df29ddd03fdc2fee6f3000f64545-r3";
const R4: &str = "a92fca6c0067df29ddd03fdc2fee6f3000f64545-r4";
const R5: &str = "a92fca6c0067df29ddd03fdc2fee6f3000f64545-r5";
const R7: &str = "a92fca6c0067df29ddd03fdc2fee6f3000f64545-r7";
const R6: &str = "a92fca6c0067df29ddd03fdc2fee6f3000f64545-r6";
const R14: &str = "a92fca6c0067df29ddd03fdc2fee6f3000f64545-r14";
const R16: &str = "a92fca6c0067df29ddd03fdc2fee6f3000f64545-r16";
const R15: &str = "a92fca6c0067df29ddd03fdc2fee6f3000f64545-r15";
const R12: &str = "a92fca6c0067df29ddd03fdc2fee6f3000f64545-r12";
const R13: &str = "a92fca6c0067df29ddd03fdc2fee6f3000f64545-r13";
const R11: &str = "a92fca6c0067df29ddd03fdc2fee6f3000f64545-r11";
const R10: &str = "a92fca6c0067df29ddd03fdc2fee6f3000f64545-r10";
const R9: &str = "a92fca6c0067df29ddd03fdc2fee6f3000f64545-r9";
const R8: &str = "a92fca6c0067df29ddd03fdc2fee6f3000f64545-r8";

#[tokio::test]
async fn saved_full_jobs_are_not_prepared_but_keep_each_report_version() {
    for version in [
        R2, R3, R4, R5, R6, R7, R8, R9, R10, R11, R12, R13, R14, R15, R16, VERSION,
    ] {
        let scratch = Scratch::new();
        let mut spec = scratch.spec();
        spec.version = version.into();
        let privileged = FakePrivileged {
            artifact_version: Some(version.into()),
            ..Default::default()
        };
        let adapter = NodeQualityAdapter::new();
        let error = adapter.prepare(&spec, &privileged).await.unwrap_err();
        assert!(error.to_string().contains("离线受控工具链"));
        assert_eq!(spec.version, version);
        assert!(privileged.calls.lock().unwrap().is_empty());
        assert!(!spec.job_dir.exists());
        std::fs::create_dir(&spec.job_dir).unwrap();
        std::fs::write(spec.job_dir.join("result.txt"), "unchanged saved report").unwrap();
        assert_eq!(
            adapter.collect(&spec).await.unwrap().unwrap().text,
            "unchanged saved report"
        );
    }
}

#[tokio::test]
async fn r4_through_r17_daily_jobs_keep_mode_targets_budget_and_saved_chapters() {
    for version in [
        R4, R5, R6, R7, R8, R9, R10, R11, R12, R13, R14, R15, R16, VERSION,
    ] {
        let scratch = Scratch::new();
        let mut spec = scratch.spec();
        spec.version = version.into();
        let targets = r#"[{"name":"private fixture","target":"127.0.0.1","port":443}]"#;
        spec.options.insert("mode".into(), "daily".into());
        spec.options.insert("daily_targets".into(), targets.into());
        spec.options
            .insert("environment_section".into(), "true".into());
        let privileged = FakePrivileged {
            artifact_version: Some(version.into()),
            ..Default::default()
        };
        let adapter = NodeQualityAdapter::new();
        let job = adapter.prepare(&spec, &privileged).await.unwrap();
        assert!(job.args.windows(2).any(|args| args == ["--mode", "daily"]));
        let target_file = spec.job_dir.join("daily-targets.json");
        assert!(
            job.args
                .windows(2)
                .any(|args| args == ["--targets-file", target_file.to_str().unwrap()])
        );
        assert_eq!(std::fs::read_to_string(target_file).unwrap(), targets);
        assert_eq!(job.memory_max.get(), 64 * 1024 * 1024);
        assert_eq!(job.tasks_max.get(), 32);
        let chapter = serde_json::json!({
            "name":"net_quality", "text":"saved daily checks", "complete":true,
            "revision":1, "collected_at":1700000000
        });
        std::fs::write(
            spec.job_dir.join("section-net_quality.json"),
            serde_json::to_vec(&chapter).unwrap(),
        )
        .unwrap();
        let chapters = adapter.collect_sections(&spec).await.unwrap();
        assert_eq!(chapters.len(), 1);
        assert_eq!(chapters[0].text, "saved daily checks");
    }
}

#[tokio::test]
async fn legacy_versions_reject_mode_options_during_prepare_and_recovery() {
    for version in [R2, R3] {
        for (option, value) in [
            ("mode", "full"),
            ("daily_targets", "[]"),
            ("environment_section", "true"),
        ] {
            let scratch = Scratch::new();
            let mut spec = scratch.spec();
            spec.version = version.into();
            spec.options.insert(option.into(), value.into());
            let privileged = FakePrivileged {
                artifact_version: Some(version.into()),
                ..Default::default()
            };
            let adapter = NodeQualityAdapter::new();
            assert!(adapter.prepare(&spec, &privileged).await.is_err());
            assert!(adapter.collect(&spec).await.is_err());
            assert!(adapter.collect_sections(&spec).await.is_err());
            assert!(privileged.calls.lock().unwrap().is_empty());
            assert!(!spec.job_dir.exists());
        }
    }
}

#[tokio::test]
async fn saved_r4_job_rejects_a_different_artifact_version() {
    let scratch = Scratch::new();
    let mut spec = scratch.spec();
    spec.version = R4.into();
    spec.options.insert("mode".into(), "daily".into());
    spec.options.insert("daily_targets".into(), "[]".into());
    let privileged = FakePrivileged::default();
    assert!(
        NodeQualityAdapter::new()
            .prepare(&spec, &privileged)
            .await
            .is_err()
    );
    assert!(!spec.job_dir.join("daily-targets.json").exists());
}
