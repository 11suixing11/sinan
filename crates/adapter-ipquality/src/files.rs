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

pub(crate) fn now() -> Result<i64> {
    Ok(i64::try_from(
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
    )?)
}

fn owner(value: &std::fs::Metadata) -> Result<u32> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(value.uid())
    }
    #[cfg(not(unix))]
    {
        let _ = value;
        anyhow::bail!("private IPQuality files require Unix")
    }
}

fn valid_metadata(
    value: &std::fs::Metadata,
    directory: bool,
    private: bool,
    expected: Option<u32>,
) -> Result<()> {
    ensure!(
        !value.file_type().is_symlink()
            && if directory {
                value.is_dir()
            } else {
                value.is_file()
            },
        "IPQuality data must be an ordinary file/directory"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        ensure!(
            value.permissions().mode() & (if private { 0o077 } else { 0o022 }) == 0,
            "IPQuality data has unsafe permissions"
        );
        ensure!(
            directory || value.nlink() == 1,
            "IPQuality hardlinks are not allowed"
        );
    }
    if let Some(expected) = expected {
        ensure!(
            owner(value)? == expected,
            "IPQuality data owner differs from its trusted workspace"
        );
    }
    Ok(())
}

pub(crate) async fn ancestors(path: &Path, deadline: Instant) -> Result<()> {
    timeout_at(deadline.min(Instant::now() + IO_TIMEOUT), async {
        for parent in path.ancestors() {
            match fs::symlink_metadata(parent).await {
                Ok(value) => ensure!(
                    value.is_dir() && !value.file_type().is_symlink(),
                    "IPQuality path ancestor must be an ordinary directory"
                ),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    })
    .await
    .context("IPQuality ancestor inspection timed out")?
}

pub(crate) async fn workspace(
    path: &Path,
    expected: Option<u32>,
    deadline: Instant,
) -> Result<Option<u32>> {
    timeout_at(deadline.min(Instant::now() + IO_TIMEOUT), async {
        let value = match fs::symlink_metadata(path).await {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            result => result?,
        };
        valid_metadata(&value, true, true, expected)?;
        ensure!(
            fs::canonicalize(path).await? == path,
            "IPQuality workspace ancestors must not be symlinks"
        );
        Ok(Some(owner(&value)?))
    })
    .await
    .context("IPQuality workspace inspection timed out")?
}

pub(crate) async fn binary(path: &Path, deadline: Instant) -> Result<u32> {
    timeout_at(deadline.min(Instant::now() + IO_TIMEOUT), async {
        let value = fs::symlink_metadata(path).await?;
        valid_metadata(&value, false, false, None)?;
        ensure!(
            fs::canonicalize(path).await? == path,
            "IPQuality binary ancestors must not be symlinks"
        );
        owner(&value)
    })
    .await
    .context("IPQuality binary inspection timed out")?
}

pub(crate) async fn read(
    path: &Path,
    limit: usize,
    expected: u32,
    private: bool,
    deadline: Instant,
) -> Result<Option<String>> {
    timeout_at(deadline.min(Instant::now() + IO_TIMEOUT), async {
        let before = match fs::symlink_metadata(path).await {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            result => result?,
        };
        valid_metadata(&before, false, private, Some(expected))?;
        ensure!(
            before.len() <= limit as u64,
            "IPQuality file exceeds its byte limit"
        );
        let file = fs::File::open(path).await?;
        let opened = file.metadata().await?;
        valid_metadata(&opened, false, private, Some(expected))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            ensure!(
                before.ino() == opened.ino() && before.dev() == opened.dev(),
                "IPQuality file identity changed"
            );
        }
        ensure!(
            opened.len() <= limit as u64,
            "IPQuality file exceeds its byte limit"
        );
        let mut bytes = Vec::new();
        file.take(limit as u64 + 1).read_to_end(&mut bytes).await?;
        let after = fs::symlink_metadata(path).await?;
        valid_metadata(&after, false, private, Some(expected))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            ensure!(
                opened.ino() == after.ino()
                    && opened.dev() == after.dev()
                    && opened.mtime() == after.mtime()
                    && opened.mtime_nsec() == after.mtime_nsec()
                    && opened.ctime() == after.ctime()
                    && opened.ctime_nsec() == after.ctime_nsec(),
                "IPQuality file changed while reading"
            );
        }
        ensure!(
            bytes.len() <= limit
                && bytes.len() as u64 == opened.len()
                && opened.len() == after.len(),
            "IPQuality file length changed or exceeded its byte limit"
        );
        Ok(Some(String::from_utf8(bytes)?))
    })
    .await
    .context("IPQuality file read timed out")?
}
