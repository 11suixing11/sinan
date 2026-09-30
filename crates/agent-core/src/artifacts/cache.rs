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
        let bytes = self.download(&artifact.url, MAX_DOWNLOAD).await?;
        verified.verify_archive(&bytes)?;
        ops.create_dir(&plugin, 0o755, None).await?;
        let archive = plugin.join(format!(".download-{}.tar.gz", Uuid::new_v4()));
        ops.write_file(&archive, &bytes, 0o600, None).await?;
        let staging = plugin.join(format!(".verified-{}", Uuid::new_v4()));
        let installed: Result<()> = async {
            ops.install_archive_files(
                &archive,
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
        if let Ok(parent) = tokio::fs::canonicalize(&plugin).await {
            if let Some(name) = archive.file_name() {
                let _ = ops.remove_path(&parent.join(name)).await;
            }
            if let Some(name) = staging.file_name() {
                let _ = ops.remove_path(&parent.join(name)).await;
            }
        }
        installed?;
        Ok(binary)
    }
}
