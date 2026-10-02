use anyhow::{Context, Result, ensure};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

// create_new must happen synchronously, before the first suspension point. An
// asynchronous open can otherwise create a file after its future is cancelled.
// Keep the original descriptor as its identity, independently of Tokio's I/O
// worker, until the owned name is either published or removed.
pub(super) struct PendingFile {
    path: PathBuf,
    identity: Option<fs::File>,
    created: Option<fs::Metadata>,
    pub file: Option<tokio::fs::File>,
    published: bool,
}

impl PendingFile {
    pub fn create(path: &Path) -> Result<Self> {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(path)?;
        let mut pending = Self {
            path: path.to_owned(),
            identity: Some(file),
            created: None,
            file: None,
            published: false,
        };
        pending.created = Some(pending.metadata()?);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            pending
                .identity
                .as_ref()
                .expect("pending identity")
                .set_permissions(fs::Permissions::from_mode(0o600))?;
        }
        pending.file = Some(tokio::fs::File::from_std(
            pending
                .identity
                .as_ref()
                .expect("pending identity")
                .try_clone()?,
        ));
        Ok(pending)
    }

    pub fn metadata(&self) -> io::Result<fs::Metadata> {
        self.identity.as_ref().expect("pending identity").metadata()
    }

    fn owns_name(&self) -> io::Result<bool> {
        // Ownership at creation is immutable evidence. Reading only the live
        // handle's UID would mistake a later chown for an object still ours.
        let Some(retained) = self.created.as_ref() else {
            return Err(io::Error::other("pending creation identity unavailable"));
        };
        let named = match fs::symlink_metadata(&self.path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error),
        };
        if !named.is_file() || named.file_type().is_symlink() {
            return Ok(false);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            Ok(named.uid() == retained.uid()
                && named.dev() == retained.dev()
                && named.ino() == retained.ino())
        }
        #[cfg(not(unix))]
        {
            let _ = retained;
            Ok(false)
        }
    }

    pub fn publish(&mut self, target: &Path) -> Result<()> {
        ensure!(self.owns_name()?, "pending report identity changed");
        super::private_metadata(&fs::symlink_metadata(&self.path)?, false)?;
        // There is deliberately no await between the last identity check and
        // commit. A cancelled blocking rename must not later replace old output.
        fs::rename(&self.path, target).context("publish report")?;
        self.published = true;
        self.close_handles();
        Ok(())
    }

    fn close_handles(&mut self) {
        self.file.take();
        self.identity.take();
    }

    pub fn cleanup(&mut self) -> Result<()> {
        if self.published || self.identity.is_none() {
            self.close_handles();
            return Ok(());
        }
        let owned = self.owns_name();
        self.close_handles();
        if owned.context("inspect owned pending report")? {
            fs::remove_file(&self.path).context("remove owned pending report")?;
        }
        Ok(())
    }
}

impl Drop for PendingFile {
    fn drop(&mut self) {
        // Explicit cleanup reports failures on ordinary error/timeout paths;
        // Drop covers an externally cancelled publication future only.
        let _ = self.cleanup();
    }
}
