use anyhow::{Context, Result, ensure};
use std::{
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    fs,
    io::AsyncReadExt,
    time::{Instant, timeout_at},
};

pub(crate) const IO_TIMEOUT: Duration = Duration::from_secs(2);

pub(crate) fn now_millis() -> Result<u64> {
    Ok(u64::try_from(
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
    )?)
}

async fn metadata(path: &Path, directory: bool, private: bool) -> Result<std::fs::Metadata> {
    let value = fs::symlink_metadata(path).await?;
    ensure!(
        !value.file_type().is_symlink()
            && if directory {
                value.is_dir()
            } else {
                value.is_file()
            },
        "diagnostic path must be an ordinary directory/file"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        ensure!(
            !private || value.permissions().mode() & 0o077 == 0,
            "diagnostic data must be private"
        );
        ensure!(
            directory || value.nlink() == 1,
            "diagnostic hardlinks are not allowed"
        );
    }
    #[cfg(not(unix))]
    anyhow::bail!("private workspace verification requires Unix");
    Ok(value)
}
pub(crate) async fn ancestors(path: &Path, deadline: Instant) -> Result<()> {
    timeout_at(deadline.min(Instant::now() + IO_TIMEOUT), async {
        for parent in path.ancestors() {
            match fs::symlink_metadata(parent).await {
                Ok(value) => ensure!(
                    value.is_dir() && !value.file_type().is_symlink(),
                    "workspace ancestor is not an ordinary directory"
                ),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    })
    .await
    .context("workspace ancestor inspection timed out")?
}
pub(crate) async fn workspace(path: &Path, deadline: Instant) -> Result<bool> {
    timeout_at(deadline.min(Instant::now() + IO_TIMEOUT), async {
        match fs::symlink_metadata(path).await {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error.into()),
            Ok(_) => (),
        }
        metadata(path, true, true).await?;
        ensure!(
            fs::canonicalize(path).await? == path,
            "workspace ancestors must not be symlinks"
        );
        Ok(true)
    })
    .await
    .context("workspace inspection timed out")?
}
pub(crate) async fn binary(path: &Path, deadline: Instant) -> Result<()> {
    timeout_at(deadline.min(Instant::now() + IO_TIMEOUT), async {
        metadata(path, false, false).await?;
        ensure!(
            fs::canonicalize(path).await? == path,
            "binary ancestors must not be symlinks"
        );
        Ok(())
    })
    .await
    .context("binary inspection timed out")?
}
pub(crate) async fn directory(path: &Path, deadline: Instant) -> Result<bool> {
    timeout_at(deadline.min(Instant::now() + IO_TIMEOUT), async {
        match fs::symlink_metadata(path).await {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error.into()),
            Ok(_) => (),
        }
        metadata(path, true, true).await?;
        Ok(true)
    })
    .await
    .context("section directory inspection timed out")?
}
pub(crate) async fn read(path: &Path, limit: usize, deadline: Instant) -> Result<Option<String>> {
    timeout_at(deadline.min(Instant::now() + IO_TIMEOUT), async {
        let before = match metadata(path, false, true).await {
            Err(error)
                if error
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound) =>
            {
                return Ok(None);
            }
            Err(error) => return Err(error),
            Ok(value) => value,
        };
        ensure!(
            before.len() <= limit as u64,
            "diagnostic file exceeds its limit"
        );
        let file = fs::File::open(path).await?;
        let opened = file.metadata().await?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::{MetadataExt, PermissionsExt};
            ensure!(
                before.ino() == opened.ino()
                    && before.dev() == opened.dev()
                    && opened.nlink() == 1
                    && opened.permissions().mode() & 0o077 == 0,
                "diagnostic file identity changed"
            );
        }
        ensure!(
            opened.is_file() && opened.len() <= limit as u64,
            "diagnostic file must be bounded and ordinary"
        );
        let mut bytes = Vec::new();
        file.take(limit as u64 + 1).read_to_end(&mut bytes).await?;
        ensure!(bytes.len() <= limit, "diagnostic file exceeds its limit");
        Ok(Some(String::from_utf8(bytes)?))
    })
    .await
    .context("diagnostic file read timed out")?
}
