use super::*;
use sinan_adapter_sdk::Execution;
use std::process::Stdio;
use tokio::io::AsyncReadExt;

async fn drain(
    mut reader: impl tokio::io::AsyncRead + Unpin,
    maximum: usize,
) -> Result<(String, bool)> {
    let mut saved = Vec::new();
    let mut buffer = [0_u8; 8192];
    let mut truncated = false;
    loop {
        let length = reader.read(&mut buffer).await?;
        if length == 0 {
            break;
        }
        let keep = length.min(maximum.saturating_sub(saved.len()));
        saved.extend_from_slice(&buffer[..keep]);
        truncated |= keep != length;
    }
    let mut text = String::from_utf8_lossy(&saved).into_owned();
    let mut limit = text.len().min(maximum);
    while !text.is_char_boundary(limit) {
        limit -= 1;
    }
    truncated |= text.len() > limit;
    text.truncate(limit);
    Ok((text, truncated))
}

pub(super) async fn execute(
    program: &Path,
    args: &[String],
    seconds: u32,
    maximum: usize,
) -> Result<Execution> {
    ensure!(
        (1..=3600).contains(&seconds) && (1..=1024 * 1024).contains(&maximum),
        "invalid command limits"
    );
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command.spawn().context("start command")?;
    let id = child.id().context("command has no process identifier")?;
    let mut stdout = tokio::spawn(drain(
        child.stdout.take().context("command has no stdout")?,
        maximum,
    ));
    let mut stderr = tokio::spawn(drain(
        child.stderr.take().context("command has no stderr")?,
        maximum,
    ));
    let status = timeout(Duration::from_secs(u64::from(seconds)), child.wait()).await;
    let timed_out = status.is_err();
    let success = matches!(&status, Ok(Ok(status)) if status.success());
    // Terminate descendants too; a finished shell may leave inherited output handles open.
    #[cfg(unix)]
    {
        let _ = Command::new("kill")
            .args(["-KILL", &format!("-{id}")])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;
    }
    #[cfg(windows)]
    if timed_out {
        let _ = Command::new("taskkill.exe")
            .args(["/PID", &id.to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;
    }
    if timed_out {
        let _ = child.kill().await;
    }
    let collected = timeout(Duration::from_secs(5), async { (&mut stdout).await? }).await;
    if collected.is_err() {
        stdout.abort();
        stderr.abort();
    }
    let (out, cut_out) = collected.context("command stdout did not close")??;
    let collected = timeout(Duration::from_secs(5), &mut stderr).await;
    if collected.is_err() {
        stderr.abort();
    }
    let (err, cut_err) = collected.context("command stderr did not close")???;
    if let Ok(status) = status {
        status?;
    }
    Ok(Execution {
        output: CommandOutput {
            success,
            stdout: out,
            stderr: err,
        },
        timed_out,
        truncated: cut_out || cut_err,
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[tokio::test]
    async fn command_timeout_kills_descendants_and_output_is_bounded() -> Result<()> {
        let timed = execute(
            Path::new("sh"),
            &["-c".into(), "sleep 30 & wait".into()],
            1,
            1024,
        )
        .await?;
        assert!(timed.timed_out && !timed.output.success);
        let output = execute(
            Path::new("sh"),
            &["-c".into(), "yes abc | head -c 100000".into()],
            5,
            64,
        )
        .await?;
        assert!(output.output.success && output.truncated);
        assert_eq!(output.output.stdout.len(), 64);
        Ok(())
    }
}
