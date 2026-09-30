use super::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

#[tokio::test]
async fn terminal_updates_survive_failed_confirmation_and_restart() -> Result<()> {
    let directory = Directory::new();
    let services = Arc::new(Services::new(JobStatus::Missing));
    let first = worker(&directory, services.clone())?;
    let id = Uuid::new_v4();
    first.finish(failure(id, "fixture failure".into(), None))?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let client = PanelClient::new(
        &format!("http://{}", listener.local_addr()?),
        "test-session",
    )?;
    let server = tokio::spawn(async move {
        for status in ["500 Internal Server Error", "204 No Content"] {
            let (mut socket, _) = listener.accept().await?;
            let mut bytes = Vec::new();
            loop {
                let mut chunk = [0_u8; 8192];
                let count = socket.read(&mut chunk).await?;
                ensure!(count > 0, "mock request closed");
                bytes.extend_from_slice(&chunk[..count]);
                if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                    let header = std::str::from_utf8(&bytes[..end])?;
                    let length: usize = header
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|value| value.trim().parse())
                        })
                        .transpose()?
                        .unwrap_or(0);
                    if bytes.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let request = std::str::from_utf8(&bytes)?;
            ensure!(
                request.starts_with(&format!("POST /api/agent/v1/diagnostics/{id} ")),
                "unexpected update path"
            );
            ensure!(
                request
                    .to_lowercase()
                    .contains("authorization: bearer test-session"),
                "missing device authorization"
            );
            socket
                .write_all(
                    format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                        .as_bytes(),
                )
                .await?;
        }
        Ok::<_, anyhow::Error>(())
    });
    assert!(first.flush(&client).await.is_err());
    drop(first);
    let recovered = worker(&directory, services)?;
    let saved: Vec<DiagnosticUpdate> = recovered.read(OUTBOX)?.unwrap();
    assert_eq!(saved[0].id, id);
    recovered.flush(&client).await?;
    let pending: Vec<DiagnosticUpdate> = recovered.read(OUTBOX)?.unwrap();
    assert!(pending.is_empty());
    recovered.accept(vec![job(id)])?;
    assert!(recovered.active()?.is_none());
    server.await??;
    Ok(())
}
