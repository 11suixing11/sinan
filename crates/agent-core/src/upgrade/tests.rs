use super::*;
use crate::system::SystemOps;
use sinan_protocol::Artifact;
use std::{os::unix::fs::PermissionsExt, path::PathBuf, sync::Mutex};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
};

struct Fixture {
    root: PathBuf,
    origin: String,
    release: Arc<Mutex<AgentRelease>>,
    requests: Arc<Mutex<Vec<String>>>,
    task: JoinHandle<()>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

impl Fixture {
    async fn new() -> Result<Self> {
        let root = std::env::temp_dir().join(format!("sinan-update-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root)?;
        let source = root.join("fixture.rs");
        std::fs::write(
            &source,
            r##"#![forbid(unsafe_code)]
fn main() { assert_eq!(std::env::args().nth(1).as_deref(), Some("--version")); println!("sinan-agent 99.0.0"); }
"##,
        )?;
        let binary = root.join("fixture");
        let output = tokio::process::Command::new("rustc")
            .args(["--edition=2024", "--crate-name", "update_fixture"])
            .arg(&source)
            .arg("-o")
            .arg(&binary)
            .output()
            .await?;
        ensure!(
            output.status.success(),
            "compile fixture: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let bytes = std::fs::read(binary)?;
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let origin = format!("http://{}", listener.local_addr()?);
        let release = Arc::new(Mutex::new(AgentRelease {
            version: "99.0.0".into(),
            artifact: Artifact {
                url: format!("{origin}/binary"),
                sha256: format!("{:x}", Sha256::digest(&bytes)),
            },
        }));
        let response_release = release.clone();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = requests.clone();
        let task = tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") && request.len() < 16 * 1024 {
                    match stream.read_u8().await {
                        Ok(byte) => request.push(byte),
                        Err(_) => break,
                    }
                }
                let request = String::from_utf8_lossy(&request).into_owned();
                let body = if request.starts_with("GET /api/agent/v1/update ") {
                    serde_json::to_vec(&*response_release.lock().unwrap()).unwrap()
                } else if request.starts_with("GET /binary ") {
                    bytes.clone()
                } else {
                    b"invalid executable".to_vec()
                };
                captured.lock().unwrap().push(request);
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                if stream.write_all(response.as_bytes()).await.is_ok() {
                    let _ = stream.write_all(&body).await;
                }
            }
        });
        Ok(Self {
            root,
            origin,
            release,
            requests,
            task,
        })
    }
}

#[tokio::test]
async fn stages_authenticated_updates_only_after_digest_format_and_version_checks() -> Result<()> {
    let fixture = Fixture::new().await?;
    let config = Config {
        agent_root: fixture.root.join("core"),
        ..Config::default()
    };
    let ops = SystemOps;
    ops.create_dir(&config.agent_root, 0o755, None).await?;
    ops.write_file(
        &config.agent_root.join("update-state.json"),
        &serde_json::to_vec(&UpgradeState {
            current: env!("CARGO_PKG_VERSION").into(),
            ..UpgradeState::default()
        })?,
        0o600,
        None,
    )
    .await?;
    let client = PanelClient::new(&fixture.origin, "update-fixture-session")?;
    let original = fixture.release.lock().unwrap().clone();
    let pending = config.agent_root.join("pending-update.json");

    fixture.release.lock().unwrap().artifact.sha256 = "0".repeat(64);
    assert!(
        check(&config, &ops, &client)
            .await
            .unwrap_err()
            .to_string()
            .contains("SHA256")
    );
    assert!(!config.agent_root.join("99.0.0").exists() && !pending.exists());

    fixture.release.lock().unwrap().artifact = Artifact {
        url: format!("{}/invalid", fixture.origin),
        sha256: format!("{:x}", Sha256::digest(b"invalid executable")),
    };
    assert!(
        check(&config, &ops, &client)
            .await
            .unwrap_err()
            .to_string()
            .contains("executable format")
    );
    assert!(!pending.exists());

    *fixture.release.lock().unwrap() = original.clone();
    fixture.release.lock().unwrap().artifact.url = "https://other.example/binary".into();
    assert!(
        check(&config, &ops, &client)
            .await
            .unwrap_err()
            .to_string()
            .contains("configured panel origin")
    );
    assert!(!pending.exists());

    *fixture.release.lock().unwrap() = original.clone();
    fixture.release.lock().unwrap().version = env!("CARGO_PKG_VERSION").into();
    assert!(
        check(&config, &ops, &client)
            .await
            .unwrap_err()
            .to_string()
            .contains("newer stable")
    );
    fixture.release.lock().unwrap().version = "99.0.1".into();
    assert!(
        check(&config, &ops, &client)
            .await
            .unwrap_err()
            .to_string()
            .contains("version check")
    );
    assert!(!pending.exists());

    *fixture.release.lock().unwrap() = original.clone();
    check(&config, &ops, &client).await?;
    let staged: PendingUpgrade = read_json(&pending)?;
    assert_eq!(staged.version, original.version);
    assert_eq!(staged.sha256, original.artifact.sha256);
    assert_eq!(
        std::fs::metadata(&pending)?.permissions().mode() & 0o777,
        0o600
    );
    assert!(state(&config)?.unwrap().trial.is_none());

    std::fs::remove_file(&pending)?;
    std::fs::write(
        config.agent_root.join("99.0.0").join(executable_name()),
        b"changed",
    )?;
    assert!(
        check(&config, &ops, &client)
            .await
            .unwrap_err()
            .to_string()
            .contains("different bytes")
    );
    assert!(!pending.exists());
    let mut failed = state(&config)?.unwrap();
    failed.failed(original.version);
    ops.write_file(
        &config.agent_root.join("update-state.json"),
        &serde_json::to_vec(&failed)?,
        0o600,
        None,
    )
    .await?;
    fixture.requests.lock().unwrap().clear();
    check(&config, &ops, &client).await?;
    let requests = fixture.requests.lock().unwrap();
    assert_eq!(
        requests.len(),
        1,
        "failed releases must not be downloaded again"
    );
    assert!(
        requests[0]
            .to_lowercase()
            .contains("authorization: bearer update-fixture-session")
    );
    assert!(!pending.exists());
    Ok(())
}
