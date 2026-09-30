#![forbid(unsafe_code)]

use sinan_adapter_nodequality::{MAX_REPORT_BYTES, NodeQualityAdapter, VERSION};
use sinan_adapter_sdk::{BoxFuture, CommandOutput, DiagnosticAdapter, DiagnosticSpec, Privileged};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "sinan-nodequality-adapter-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn spec(&self) -> DiagnosticSpec {
        DiagnosticSpec {
            id: "12345678-1234-1234-1234-123456789abc".into(),
            version: VERSION.into(),
            binary_path: self.0.join("nodequality"),
            job_dir: self.0.join("job"),
            timeout_secs: 1800,
            options: BTreeMap::new(),
        }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[derive(Default)]
struct FakePrivileged {
    calls: Mutex<Vec<Vec<String>>>,
    invalid_version: bool,
}

impl Privileged for FakePrivileged {
    fn execute<'a>(&'a self, _: &'a Path, args: &'a [String]) -> BoxFuture<'a, CommandOutput> {
        Box::pin(async move {
            self.calls.lock().unwrap().push(args.to_vec());
            Ok(CommandOutput {
                success: true,
                stdout: if self.invalid_version {
                    "another program".into()
                } else {
                    format!("nodequality {VERSION}\n")
                },
                stderr: String::new(),
            })
        })
    }

    fn create_dir<'a>(
        &'a self,
        path: &'a Path,
        mode: u32,
        _: Option<&'a str>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            assert_eq!(mode, 0o700);
            tokio::fs::create_dir_all(path).await?;
            Ok(())
        })
    }

    fn write_file<'a>(
        &'a self,
        _: &'a Path,
        _: &'a [u8],
        _: u32,
        _: Option<&'a str>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async { anyhow::bail!("unexpected write") })
    }

    fn atomic_symlink<'a>(&'a self, _: &'a Path, _: &'a Path) -> BoxFuture<'a, ()> {
        Box::pin(async { anyhow::bail!("unexpected symlink") })
    }

    fn remove_symlink<'a>(&'a self, _: &'a Path) -> BoxFuture<'a, ()> {
        Box::pin(async { anyhow::bail!("unexpected symlink removal") })
    }

    fn install_archive<'a>(&'a self, _: &'a Path, _: &'a Path, _: &'a str) -> BoxFuture<'a, ()> {
        Box::pin(async { anyhow::bail!("unexpected archive install") })
    }
}

#[tokio::test]
async fn prepare_only_verifies_version_and_builds_a_fixed_service_command() {
    let scratch = Scratch::new();
    let spec = scratch.spec();
    let privileged = FakePrivileged::default();
    let job = NodeQualityAdapter::new()
        .prepare(&spec, &privileged)
        .await
        .unwrap();
    assert_eq!(job.unit, format!("sinan-diagnostic-{}.service", spec.id));
    assert_eq!(job.program, spec.binary_path);
    assert_eq!(job.working_directory, spec.job_dir);
    assert_eq!(job.timeout_secs, 1800);
    assert_eq!(
        job.args,
        vec![
            "--workspace",
            spec.job_dir.to_str().unwrap(),
            "--ip-version",
            "both",
            "--network-mode",
            "low",
            "--upload-report",
            "false",
        ]
    );
    assert_eq!(
        *privileged.calls.lock().unwrap(),
        vec![vec!["--version".to_string()]]
    );
}

#[tokio::test]
async fn prepare_rejects_unpinned_versions_unknown_options_and_expansion() {
    let scratch = Scratch::new();
    let privileged = FakePrivileged::default();
    for case in 0..12 {
        let mut spec = scratch.spec();
        match case {
            0 => spec.version = "latest".into(),
            1 => {
                spec.options.insert("command".into(), "id".into());
            }
            2 => {
                spec.options.insert("ip_version".into(), "$(id)".into());
            }
            3 => spec.job_dir = scratch.0.join("%n"),
            4 => spec.id = "bad/service".into(),
            5 => spec.timeout_secs = 3601,
            6 => spec.job_dir = PathBuf::from("/"),
            7 => spec.job_dir = scratch.0.join("space path"),
            8 => spec.job_dir = scratch.0.join("wild*card"),
            9 => spec.job_dir = scratch.0.join("question?mark"),
            10 => {
                spec.options.insert("upload_report".into(), "yes".into());
            }
            11 => spec.version = "a92fca6c0067df29ddd03fdc2fee6f3000f64545".into(),
            _ => unreachable!(),
        }
        assert!(
            NodeQualityAdapter::new()
                .prepare(&spec, &privileged)
                .await
                .is_err()
        );
    }
    assert!(privileged.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn prepare_uploads_only_with_an_explicit_true_option() {
    let scratch = Scratch::new();
    let privileged = FakePrivileged::default();
    for option in ["true", "false"] {
        let mut spec = scratch.spec();
        spec.options.insert("upload_report".into(), option.into());
        let job = NodeQualityAdapter::new()
            .prepare(&spec, &privileged)
            .await
            .unwrap();
        assert_eq!(&job.args[job.args.len() - 2..], ["--upload-report", option]);
    }
}

#[tokio::test]
async fn prepare_rejects_an_artifact_that_does_not_match_its_version() {
    let scratch = Scratch::new();
    let privileged = FakePrivileged {
        invalid_version: true,
        ..Default::default()
    };
    assert!(
        NodeQualityAdapter::new()
            .prepare(&scratch.spec(), &privileged)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn collect_preserves_local_report_when_online_upload_failed() {
    let scratch = Scratch::new();
    let spec = scratch.spec();
    std::fs::create_dir(&spec.job_dir).unwrap();
    std::fs::write(
        spec.job_dir.join("result.txt"),
        "硬件质量\nIP 质量\nHTTP 状态：403\n",
    )
    .unwrap();
    let output = NodeQualityAdapter::new()
        .collect(&spec)
        .await
        .unwrap()
        .unwrap();
    assert!(output.text.contains("IP 质量"));
    assert_eq!(output.report_url, None);
    std::fs::write(
        spec.job_dir.join("report-url.txt"),
        "https://nodequality.com/r/report_TOKEN-123\n",
    )
    .unwrap();
    assert_eq!(
        NodeQualityAdapter::new()
            .collect(&spec)
            .await
            .unwrap()
            .unwrap()
            .report_url,
        Some("https://nodequality.com/r/report_TOKEN-123".into())
    );
}

#[tokio::test]
async fn collect_rejects_a_remote_or_malformed_url_and_oversized_report() {
    let scratch = Scratch::new();
    let spec = scratch.spec();
    std::fs::create_dir(&spec.job_dir).unwrap();
    std::fs::write(spec.job_dir.join("result.txt"), "actual local report").unwrap();
    for value in [
        "https://evil.example/r/a",
        "https://nodequality.com/r/a?next=x",
        "https://nodequality.com/r/",
    ] {
        std::fs::write(spec.job_dir.join("report-url.txt"), value).unwrap();
        assert!(NodeQualityAdapter::new().collect(&spec).await.is_err());
    }
    std::fs::remove_file(spec.job_dir.join("report-url.txt")).unwrap();
    std::fs::write(
        spec.job_dir.join("result.txt"),
        vec![b'a'; MAX_REPORT_BYTES as usize + 1],
    )
    .unwrap();
    assert!(NodeQualityAdapter::new().collect(&spec).await.is_err());
}

#[tokio::test]
async fn collect_never_treats_a_log_or_stale_url_as_a_completed_report() {
    let scratch = Scratch::new();
    let spec = scratch.spec();
    assert!(
        NodeQualityAdapter::new()
            .collect(&spec)
            .await
            .unwrap()
            .is_none()
    );
    std::fs::create_dir(&spec.job_dir).unwrap();
    std::fs::write(spec.job_dir.join("log.txt"), "mount failed").unwrap();
    std::fs::write(
        spec.job_dir.join("report-url.txt"),
        "https://nodequality.com/r/stale",
    )
    .unwrap();
    assert!(
        NodeQualityAdapter::new()
            .collect(&spec)
            .await
            .unwrap()
            .is_none()
    );
}

#[cfg(unix)]
#[tokio::test]
async fn collect_does_not_follow_output_symlinks() {
    let scratch = Scratch::new();
    let spec = scratch.spec();
    std::fs::create_dir(&spec.job_dir).unwrap();
    let secret = scratch.0.join("private-file");
    std::fs::write(&secret, "private").unwrap();
    std::os::unix::fs::symlink(secret, spec.job_dir.join("result.txt")).unwrap();
    assert!(NodeQualityAdapter::new().collect(&spec).await.is_err());
}
