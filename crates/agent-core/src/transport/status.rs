use super::Runtime;
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::atomic::Ordering,
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
    time::timeout,
};

pub(super) struct BoundSocket {
    listener: UnixListener,
    path: PathBuf,
}

impl Drop for BoundSocket {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

pub(super) async fn bind(path: &Path) -> Result<BoundSocket> {
    use std::os::unix::fs::{FileTypeExt, PermissionsExt};
    let parent = path.parent().context("status socket has no parent")?;
    tokio::fs::create_dir_all(parent).await?;
    tokio::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700)).await?;
    if let Ok(metadata) = tokio::fs::symlink_metadata(path).await {
        anyhow::ensure!(
            metadata.file_type().is_socket(),
            "status path already exists and is not a socket"
        );
        anyhow::ensure!(
            UnixStream::connect(path).await.is_err(),
            "agent status socket is already active"
        );
        tokio::fs::remove_file(path).await?;
    }
    let listener = UnixListener::bind(path)?;
    tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).await?;
    Ok(BoundSocket {
        listener,
        path: path.to_path_buf(),
    })
}

pub(super) async fn serve(bound: BoundSocket, runtime: Runtime) -> Result<()> {
    loop {
        let (mut socket, _) = bound.listener.accept().await?;
        let snapshot = snapshot(&runtime)?;
        let mut bytes = serde_json::to_vec(&snapshot)?;
        bytes.push(b'\n');
        if let Err(error) = timeout(Duration::from_secs(2), socket.write_all(&bytes)).await {
            tracing::debug!(%error, "status client timed out");
        }
    }
}

fn snapshot(runtime: &Runtime) -> Result<Value> {
    let applied = runtime.applied()?;
    let state = runtime
        .state
        .lock()
        .map_err(|_| anyhow::anyhow!("state lock poisoned"))?;
    let mut healthy = BTreeMap::new();
    for module in runtime.modules.iter() {
        healthy.insert(
            module.clone(),
            state
                .get_json::<bool>(&format!("health:{module}"))?
                .unwrap_or(false),
        );
    }
    Ok(json!({
        "connected": runtime.connected.load(Ordering::Relaxed),
        "applied": applied, "healthy": healthy, "pending_batches": state.pending_usage_count()?,
    }))
}

pub async fn status(path: &Path) -> Result<Value> {
    timeout(Duration::from_secs(5), async {
        let socket = UnixStream::connect(path)
            .await
            .context("connect to running agent status socket")?;
        let mut bytes = Vec::new();
        socket.take(65_537).read_to_end(&mut bytes).await?;
        anyhow::ensure!(bytes.len() <= 65_536, "status response exceeds size limit");
        Ok(serde_json::from_slice(&bytes)?)
    })
    .await?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::State;
    use std::sync::{atomic::AtomicBool, Arc, Mutex};
    use tokio::task::JoinSet;

    #[tokio::test]
    async fn status_socket_is_private_reports_state_and_cleans_up_on_abort() -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let directory =
            std::path::PathBuf::from("/tmp").join(format!("sn-status-{}", uuid::Uuid::new_v4()));
        let socket = directory.join("status.sock");
        let runtime = Runtime {
            state: Arc::new(Mutex::new(State::open(&directory.join("state.db"))?)),
            modules: Arc::new(vec![]),
            connected: Arc::new(AtomicBool::new(true)),
        };
        let bound = bind(&socket).await?;
        assert_eq!(
            std::fs::metadata(&socket)?.permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::metadata(&directory)?.permissions().mode() & 0o777,
            0o700
        );
        assert!(bind(&socket).await.is_err());
        let mut tasks = JoinSet::new();
        tasks.spawn(serve(bound, runtime));
        let value = status(&socket).await?;
        assert_eq!(value["connected"], true);
        assert_eq!(value["applied"], json!({}));
        assert_eq!(value["pending_batches"], 0);
        tasks.abort_all();
        while tasks.join_next().await.is_some() {}
        assert!(!socket.exists());
        std::fs::remove_dir_all(directory)?;
        Ok(())
    }
}
