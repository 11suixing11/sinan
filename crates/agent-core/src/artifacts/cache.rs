use super::*;

impl PanelClient {
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
        self.validate_url(&artifact.url)?;
        let expected = normalized_hash(&artifact.sha256)?;
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
                let marker = directory.join(MARKER);
                let metadata = tokio::fs::symlink_metadata(&marker)
                    .await
                    .context("existing artifact has no cache metadata; refusing to overwrite")?;
                ensure!(
                    metadata.is_file() && metadata.len() <= 4096,
                    "invalid artifact cache metadata"
                );
                let cached: InstalledArtifact =
                    serde_json::from_slice(&tokio::fs::read(marker).await?)?;
                ensure!(
                    cached.archive_sha256 == expected,
                    "existing artifact version has a different SHA256; refusing to overwrite"
                );
                ensure!(
                    file_digest(&binary).await? == cached.binary_sha256,
                    "cached artifact binary was modified"
                );
                ensure!(
                    cached.auxiliary_sha256.len() == descriptor.auxiliary_files.len(),
                    "cached artifact file set changed"
                );
                for name in &descriptor.auxiliary_files {
                    let path = directory.join(name);
                    ensure!(
                        tokio::fs::symlink_metadata(&path).await?.is_file(),
                        "cached auxiliary file is not ordinary"
                    );
                    ensure!(
                        cached.auxiliary_sha256.get(name) == Some(&file_digest(&path).await?),
                        "cached auxiliary artifact was modified"
                    );
                }
                return Ok(binary);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let bytes = self.download(&artifact.url, MAX_DOWNLOAD).await?;
        ensure!(digest(&bytes) == expected, "artifact SHA256 mismatch");
        ops.create_dir(&plugin, 0o755, None).await?;
        let archive = plugin.join(format!(".download-{}.tar.gz", Uuid::new_v4()));
        ops.write_file(&archive, &bytes, 0o600, None).await?;
        let installed = ops
            .install_archive_files(
                &archive,
                &directory,
                &descriptor.binary_name,
                &descriptor.auxiliary_files,
            )
            .await;
        #[cfg(unix)]
        let _ = ops
            .execute(
                Path::new("rm"),
                &[
                    "-f".into(),
                    "--".into(),
                    archive.to_string_lossy().into_owned(),
                ],
            )
            .await;
        #[cfg(windows)]
        let _ = ops
            .execute(
                Path::new("powershell.exe"),
                &[
                    "-NoProfile".into(),
                    "-NonInteractive".into(),
                    "-Command".into(),
                    format!(
                        "Remove-Item -LiteralPath {} -Force",
                        crate::system::ps_quote(&archive.to_string_lossy())
                    ),
                ],
            )
            .await;
        installed?;
        let mut auxiliary_sha256 = std::collections::BTreeMap::new();
        for name in &descriptor.auxiliary_files {
            auxiliary_sha256.insert(name.clone(), file_digest(&directory.join(name)).await?);
        }
        let marker = InstalledArtifact {
            archive_sha256: expected,
            binary_sha256: file_digest(&binary).await?,
            auxiliary_sha256,
        };
        ops.write_file(
            &directory.join(MARKER),
            &serde_json::to_vec(&marker)?,
            0o644,
            None,
        )
        .await?;
        Ok(binary)
    }
}

async fn ensure_ordinary_directory_if_present(path: &Path) -> Result<()> {
    match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) => ensure!(
            metadata.is_dir(),
            "artifact plugin path is not an ordinary directory"
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

async fn file_digest(path: &Path) -> Result<String> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut length = 0_u64;
    loop {
        let count = file.read(&mut buffer).await?;
        if count == 0 {
            break;
        }
        length += count as u64;
        ensure!(
            length <= 256 * 1024 * 1024,
            "cached artifact exceeds size limit"
        );
        hash.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
