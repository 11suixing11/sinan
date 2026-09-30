use super::*;
use crate::system::SystemOps;
use std::path::{Path, PathBuf};

struct Directory(PathBuf);
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct ToolSource {
    cancellation: Arc<Cancellation>,
    runtime: tokio::runtime::Runtime,
    script: String,
    finished: mpsc::Sender<()>,
}

impl Source for ToolSource {
    fn collect(&mut self) -> Result<(StaticInfo, Metrics)> {
        let args = vec!["-c".into(), self.script.clone()];
        let result = self.cancellation.run(
            &self.runtime,
            SystemOps.execute_bounded(Path::new("sh"), &args, 10, 1024),
        );
        let _ = self.finished.send(());
        result?;
        Ok((StaticInfo::default(), Metrics::default()))
    }
}

#[tokio::test]
async fn sampling_shutdown_confirms_tool_group_cleanup_before_returning() -> Result<()> {
    let directory =
        Directory(std::env::temp_dir().join(format!("sn-telemetry-stop-{}", Uuid::new_v4())));
    std::fs::create_dir(&directory.0)?;
    let ready = directory.0.join("ready");
    let escaped = directory.0.join("escaped");
    // The delayed write belongs to a descendant of the collector's tool, not
    // just its direct child. Keep the regression independent of systemd.
    let script = format!(
        "(touch '{}'; sleep 2; touch '{}') & wait",
        ready.display(),
        escaped.display(),
    );
    let (finished, complete) = mpsc::channel();
    let sampling = Sampling::start_with_cancellation(
        move |cancellation| {
            Ok(Box::new(ToolSource {
                cancellation,
                runtime: tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?,
                script,
                finished,
            }))
        },
        Control {
            interval: Duration::from_millis(10),
            offset_ms: 0,
            minimum_timestamp: 0,
            enabled: true,
        },
    )?;
    tokio::time::timeout(Duration::from_secs(5), async {
        while !ready.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await?;
    let cancellation = sampling.cancellation.clone();
    assert!(cancellation.is_active());
    let started = Instant::now();
    drop(sampling);
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(cancellation.requested());
    assert!(
        !cancellation.is_active(),
        "tool cleanup was still pending at shutdown return"
    );
    // The cleanup is already confirmed, rather than depending on the Agent's
    // runtime being polled again after its terminal retirement return.
    complete.recv_timeout(Duration::from_secs(1))?;
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert!(
        !escaped.exists(),
        "telemetry tool descendant survived shutdown"
    );
    Ok(())
}
