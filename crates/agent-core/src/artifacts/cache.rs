use super::*;
use tokio::io::AsyncWriteExt;

struct DownloadFile {
    path: PathBuf,
    file: Option<tokio::fs::File>,
}

impl DownloadFile {
    fn create(parent: &Path) -> Result<Self> {
        let path = parent.join(format!(".download-{}.tar.gz", Uuid::new_v4()));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        // Create synchronously so cancellation cannot orphan a pending file open.
        let file = options.open(&path)?;
        Ok(Self {
            path,
            file: Some(tokio::fs::File::from_std(file)),
        })
    }

    async fn close(&mut self) -> Result<()> {
        let mut file = self
            .file
            .take()
            .context("download file is already closed")?;
        file.flush().await?;
        file.sync_all().await?;
        Ok(())
    }
}

impl Drop for DownloadFile {
    fn drop(&mut self) {
        drop(self.file.take());
        // Only unlink the uniquely created file; never traverse a replacement.
        if let Err(error) = std::fs::remove_file(&self.path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(%error, "cannot remove private artifact download");
        }
    }
}

impl PanelClient {
    async fn download_archive(
        &self,
        value: &str,
        verified: &VerifiedArtifact,
        parent: &Path,
    ) -> Result<DownloadFile> {
        let expected_size = verified.metadata().archive_size;
        ensure!(
            (1..=MAX_DOWNLOAD as u64).contains(&expected_size),
            "signed artifact exceeds download size limit"
        );
        let url = self.validate_url(value)?;
        ensure_ordinary_directory_if_present(parent).await?;
        let parent = tokio::fs::canonicalize(parent).await?;
        ensure_ordinary_directory_if_present(&parent).await?;
        let mut download = DownloadFile::create(&parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            download
                .file
                .as_ref()
                .context("download file is closed")?
                .set_permissions(std::fs::Permissions::from_mode(0o600))
                .await?;
        }
        let response = self
            .client
            .get(url)
            .bearer_auth(&self.session_token)
            .send()
            .await?;
        ensure!(
            response.status().is_success(),
            "panel download returned HTTP {}",
            response.status()
        );
        if let Some(length) = response.content_length() {
            ensure!(
                length <= MAX_DOWNLOAD as u64 && length == expected_size,
                "download Content-Length differs from signed archive size"
            );
        }
        let mut length = 0_u64;
        let mut hash = Sha256::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let next = length
                .checked_add(chunk.len() as u64)
                .context("download length overflow")?;
            ensure!(
                next <= expected_size && next <= MAX_DOWNLOAD as u64,
                "download exceeds signed archive size limit"
            );
            download
                .file
                .as_mut()
                .context("download file is closed")?
                .write_all(&chunk)
                .await?;
            hash.update(&chunk);
            length = next;
        }
        ensure!(length == expected_size, "download archive was truncated");
        ensure!(
            format!("{:x}", hash.finalize()) == verified.sha256(),
            "download archive SHA256 differs from signed release"
        );
        download.close().await?;
        Ok(download)
    }

    pub async fn ensure_artifact(
        &self,
        artifact: &Artifact,
        version: &str,
        descriptor: &Descriptor,
        install_root: &Path,
        ops: &dyn Privileged,
    ) -> Result<PathBuf> {
        ensure!(
            install_root.is_absolute(),
            "artifact installation root must be absolute"
        );
        ensure!(
            safe_component(version)
                && safe_component(&descriptor.plugin_name)
                && safe_component(&descriptor.binary_name)
                && descriptor.auxiliary_files.len() <= 7
                && descriptor
                    .auxiliary_files
                    .iter()
                    .all(|name| safe_component(name)),
            "invalid artifact version or descriptor path"
        );
        let verified = self.verify_artifact(artifact, version, descriptor)?;
        ensure_ordinary_directory_if_present(install_root).await?;
        let plugin = install_root.join(&descriptor.plugin_name);
        let directory = plugin.join(version);
        let binary = directory.join(&descriptor.binary_name);
        ensure_ordinary_directory_if_present(&plugin).await?;
        match tokio::fs::symlink_metadata(&directory).await {
            Ok(metadata) => {
                ensure!(
                    metadata.is_dir(),
                    "cached artifact directory is not an ordinary directory"
                );
                ensure!(
                    tokio::fs::symlink_metadata(&binary).await?.is_file(),
                    "cached artifact is not an ordinary file"
                );
                let proof = verification::read_proof(&directory).await?;
                let cached =
                    verification::signed_artifact(&proof, descriptor, version, self.keys()?)?;
                ensure!(
                    cached.sha256() == verified.sha256(),
                    "existing artifact version has different signed contents"
                );
                verification::verify_file(&binary, &verified).await?;
                return Ok(binary);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        ops.create_dir(&plugin, 0o755, None).await?;
        ensure_ordinary_directory_if_present(&plugin).await?;
        let download = self
            .download_archive(&artifact.url, &verified, &plugin)
            .await?;
        let staging = plugin.join(format!(".verified-{}", Uuid::new_v4()));
        let installed: Result<()> = async {
            ops.install_archive_files(
                &download.path,
                &staging,
                &descriptor.binary_name,
                &descriptor.auxiliary_files,
            )
            .await?;
            verification::verify_file(&staging.join(&descriptor.binary_name), &verified).await?;
            let proof = artifact
                .proof
                .as_ref()
                .context("artifact has no signed proof")?;
            for (name, bytes) in [
                ("release.json", proof.metadata_json.as_bytes()),
                ("SHA256SUMS", proof.checksums.as_bytes()),
                ("SHA256SUMS.minisig", proof.signature.as_bytes()),
            ] {
                ops.write_file(&staging.join(name), bytes, 0o644, None)
                    .await?;
            }
            let marker = InstalledArtifact {
                proof: Some(proof.clone()),
            };
            ops.write_file(
                &staging.join(MARKER),
                &serde_json::to_vec(&marker)?,
                0o644,
                None,
            )
            .await?;
            ops.publish_directory(&staging, &directory).await?;
            verify_expected(&binary, descriptor, version, self.keys()?).await?;
            let actual = verification::signed_artifact(
                &verification::read_proof(&directory).await?,
                descriptor,
                version,
                self.keys()?,
            )?;
            ensure!(
                actual.sha256() == verified.sha256(),
                "artifact publication raced with different contents"
            );
            Ok(())
        }
        .await;
        // Resolve only the already controlled parent, never a replacement object.
        if let Ok(parent) = tokio::fs::canonicalize(&plugin).await
            && let Some(name) = staging.file_name()
        {
            let _ = ops.remove_path(&parent.join(name)).await;
        }
        installed?;
        Ok(binary)
    }
}

#[cfg(test)]
#[path = "cache/tests.rs"]
mod tests;
