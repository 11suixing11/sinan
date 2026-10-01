use super::*;
use serde::{Deserialize, Serialize};

const MAX_INVENTORY: usize = 256 * 1024;
const INVENTORY_FILE: &str = "inventory.json";

#[cfg(test)]
#[path = "../../../protocol/tests/support/release.rs"]
mod signing;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LocalInventory {
    paths: Vec<String>,
}

pub(super) async fn stored_paths(
    directory: &Path,
    verified: &VerifiedRelease,
) -> Result<BTreeSet<String>> {
    let signed: BTreeSet<_> = verified
        .metadata()
        .artifacts
        .iter()
        .map(|entry| canonical_path(&entry.name, &entry.version, &entry.arch))
        .collect::<std::result::Result<_, _>>()?;
    let path = directory.join(INVENTORY_FILE);
    match tokio::fs::symlink_metadata(&path).await {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(signed),
        Err(error) => return Err(error.into()),
        Ok(_) => (),
    }
    let inventory: LocalInventory =
        serde_json::from_slice(&ordinary_bytes(&path, MAX_INVENTORY).await?)?;
    let paths: BTreeSet<_> = inventory.paths.iter().cloned().collect();
    ensure!(
        !paths.is_empty() && paths.len() == inventory.paths.len() && paths.is_subset(&signed),
        "local inventory must be a nonempty unique subset of signed paths"
    );
    Ok(paths)
}

pub(super) async fn existing_bytes(path: &Path, maximum: usize) -> Result<Option<Vec<u8>>> {
    if !ordinary_directory(path.parent().context("release file parent")?, false).await? {
        return Ok(None);
    }
    let metadata = match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    ensure!(
        metadata.is_file() && !metadata.is_symlink(),
        "existing release payload must be an ordinary file"
    );
    if metadata.len() > maximum as u64 {
        return Ok(None);
    }
    Ok(Some(ordinary_bytes(path, maximum).await?))
}

pub(super) async fn publish_agent<A, Aut>(
    release: &StoredRelease,
    artifact: &VerifiedArtifact,
    bytes: &[u8],
    mut authorize: A,
) -> ApiResult<()>
where
    A: FnMut() -> Aut,
    Aut: Future<Output = ApiResult<()>>,
{
    let root = release.directory.parent().context("release parent")?;
    let staging = root.join(format!(".staging-{}", Uuid::new_v4()));
    ordinary_directory(root, false).await.map_err(invalid)?;
    tokio::fs::create_dir(&staging).await.map_err(invalid)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(&staging, std::fs::Permissions::from_mode(0o700))
            .await
            .map_err(invalid)?;
    }
    let result = async {
        verify_payload(artifact, bytes).map_err(invalid)?;
        let staged = staging.join(artifact.path());
        let parent = staged.parent().context("artifact parent")?;
        ordinary_directory(parent, true).await.map_err(invalid)?;
        synced_write(&staged, bytes).await.map_err(invalid)?;
        let mut paths = release.paths.clone();
        paths.insert(artifact.path().into());
        let inventory = serde_json::to_vec(&LocalInventory {
            paths: paths.into_iter().collect(),
        })
        .map_err(invalid)?;
        synced_write(&staging.join(INVENTORY_FILE), &inventory)
            .await
            .map_err(invalid)?;
        sync_directory(parent).await.map_err(invalid)?;
        sync_directory(parent.parent().context("component parent")?)
            .await
            .map_err(invalid)?;
        sync_directory(&staging).await.map_err(invalid)?;
        // A revoked or expired token cannot publish a payload after its download.
        authorize().await?;
        checked(
            proof_at(&release.directory).await.map_err(invalid)? == release.proof
                && stored_paths(&release.directory, &release.verified)
                    .await
                    .map_err(invalid)?
                    == release.paths,
            "stored release changed during Agent download",
        )?;
        let output = release.directory.join(artifact.path());
        let parent = output.parent().context("artifact parent")?;
        ordinary_directory(parent, true).await.map_err(invalid)?;
        existing_bytes(&output, MAX_ARTIFACT)
            .await
            .map_err(invalid)?;
        existing_bytes(&release.directory.join(INVENTORY_FILE), MAX_INVENTORY)
            .await
            .map_err(invalid)?;
        tokio::fs::rename(&staged, &output).await.map_err(invalid)?;
        sync_directory(parent).await.map_err(invalid)?;
        sync_directory(parent.parent().context("component parent")?)
            .await
            .map_err(invalid)?;
        // Only this signed Agent is added; unrelated payloads and proof bytes stay intact.
        tokio::fs::rename(
            staging.join(INVENTORY_FILE),
            release.directory.join(INVENTORY_FILE),
        )
        .await
        .map_err(invalid)?;
        sync_directory(&release.directory).await.map_err(invalid)?;
        Ok(())
    }
    .await;
    let _ = tokio::fs::remove_dir_all(&staging).await;
    result
}

fn installer_valid(verified: &VerifiedRelease, bytes: &[u8]) -> bool {
    bytes.len() <= MAX_INSTALLER
        && verified.checksum("install.sh") == Some(format!("{:x}", Sha256::digest(bytes)).as_str())
}

pub(super) async fn store<F, Fut>(
    state: &AppState,
    tag: &str,
    proof: ReleaseProof,
    verified: VerifiedRelease,
    requested: BTreeSet<String>,
    old: Option<&StoredRelease>,
    mut asset_bytes: F,
) -> ApiResult<usize>
where
    F: FnMut(String, usize) -> Fut,
    Fut: Future<Output = Result<Vec<u8>>>,
{
    let root = state.config.data_dir.join("artifacts/releases");
    ordinary_directory(&root, true).await.map_err(invalid)?;
    let directory = root.join(format!(".staging-{}", Uuid::new_v4()));
    tokio::fs::create_dir(&directory)
        .await
        .map_err(anyhow::Error::from)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))
            .await
            .map_err(anyhow::Error::from)?;
    }
    let result = async {
        let mut paths = old.map(|release| release.paths.clone()).unwrap_or_default();
        let mut files = Vec::new();
        if old.is_none() {
            for (name, bytes) in [
                ("release.json", proof.metadata_json.as_bytes()),
                ("SHA256SUMS", proof.checksums.as_bytes()),
                ("SHA256SUMS.minisig", proof.signature.as_bytes()),
            ] {
                synced_write(&directory.join(name), bytes).await?;
            }
        }
        let installer = match old {
            Some(release) => {
                existing_bytes(&release.directory.join("install.sh"), MAX_INSTALLER).await?
            }
            None => None,
        };
        if !installer.is_some_and(|bytes| installer_valid(&verified, &bytes)) {
            let installer = asset_bytes("install.sh".into(), MAX_INSTALLER).await?;
            ensure!(
                installer_valid(&verified, &installer),
                "installer digest differs"
            );
            synced_write(&directory.join("install.sh"), &installer).await?;
            files.push("install.sh".to_owned());
        }
        let mut parents = BTreeSet::new();
        for entry in &verified.metadata().artifacts {
            let artifact = verified.artifact(&entry.name, &entry.version, &entry.arch)?;
            let selected = requested.contains(artifact.path());
            if !selected && !paths.contains(artifact.path()) {
                continue;
            }
            let bytes = match old {
                Some(release) => {
                    existing_bytes(&release.directory.join(artifact.path()), MAX_ARTIFACT).await?
                }
                None => None,
            };
            if bytes.is_some_and(|bytes| verify_payload(&artifact, &bytes).is_ok()) {
                paths.insert(artifact.path().into());
                continue;
            }
            if !selected {
                // Keep stale files on disk, but do not advertise unusable unrequested ABIs.
                paths.remove(artifact.path());
                continue;
            }
            let bytes =
                asset_bytes(entry.asset_name.clone(), entry.archive_size.try_into()?).await?;
            verify_payload(&artifact, &bytes)?;
            let output = directory.join(artifact.path());
            let parent = output.parent().context("artifact parent")?;
            ordinary_directory(parent, true).await?;
            synced_write(&output, &bytes).await?;
            parents.insert(parent.to_owned());
            parents.insert(parent.parent().context("component parent")?.to_owned());
            files.push(artifact.path().into());
            paths.insert(artifact.path().into());
        }
        let count = paths.len();
        if old.is_some_and(|release| release.paths == paths) && files.is_empty() {
            return Ok(count);
        }
        let inventory = serde_json::to_vec(&LocalInventory {
            paths: paths.into_iter().collect(),
        })?;
        synced_write(&directory.join(INVENTORY_FILE), &inventory).await?;
        for parent in parents.iter().rev() {
            sync_directory(parent).await?;
        }
        sync_directory(&directory).await?;
        ensure!(
            ordinary_directory(&root, false).await?,
            "release root disappeared"
        );
        if let Some(release) = old {
            ensure!(
                ordinary_directory(&release.directory, false).await?,
                "release directory disappeared"
            );
            for path in files {
                let output = release.directory.join(&path);
                let parent = output.parent().context("artifact parent")?;
                ordinary_directory(parent, true).await?;
                existing_bytes(&output, MAX_ARTIFACT).await?;
                tokio::fs::rename(directory.join(&path), &output).await?;
                sync_directory(parent).await?;
                if let Some(component) = parent.parent()
                    && component != root
                {
                    sync_directory(component).await?;
                }
            }
            // The inventory is published last, after every advertised payload is durable.
            existing_bytes(&release.directory.join(INVENTORY_FILE), MAX_INVENTORY).await?;
            tokio::fs::rename(
                directory.join(INVENTORY_FILE),
                release.directory.join(INVENTORY_FILE),
            )
            .await?;
            sync_directory(&release.directory).await?;
        } else {
            let output = root.join(tag);
            match tokio::fs::symlink_metadata(&output).await {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                _ => anyhow::bail!("release destination appeared during import"),
            }
            tokio::fs::rename(&directory, &output).await?;
            sync_directory(&root).await?;
        }
        Ok::<usize, anyhow::Error>(count)
    }
    .await;
    let _ = tokio::fs::remove_dir_all(&directory).await;
    result.map_err(invalid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn expired_authorization_cannot_publish_a_staged_agent() -> Result<()> {
        let root = std::env::temp_dir()
            .canonicalize()?
            .join(format!("sinan-agent-authorization-{}", Uuid::new_v4()));
        let directory = root.join("agent-v0.3.0");
        std::fs::create_dir_all(&directory)?;
        let bytes = b"signed Agent fixture";
        let entry = signing::entry("agent", "0.3.0", "sinan-agent", "raw", bytes, bytes);
        let arch = entry.arch.clone();
        let proof = signing::signed_release(vec![(entry, bytes.to_vec())]);
        signing::install_proof(&directory, &proof);
        let verified = verify_release(&proof, &signing::trusted_keys())?;
        let artifact = verified.artifact("agent", "0.3.0", &arch)?;
        let output = directory.join(artifact.path());
        std::fs::create_dir_all(output.parent().context("artifact parent")?)?;
        std::fs::write(&output, b"previous damaged Agent")?;
        let release = StoredRelease {
            directory: directory.clone(),
            proof,
            verified,
            paths: BTreeSet::from([artifact.path().into()]),
        };
        let result = publish_agent(&release, &artifact, bytes, || async {
            Err(ApiError::Unauthorized)
        })
        .await;
        ensure!(
            matches!(result, Err(ApiError::Unauthorized)),
            "authorization must fail"
        );
        ensure!(
            std::fs::read(output)? == b"previous damaged Agent",
            "payload must stay unchanged"
        );
        ensure!(
            !directory.join(INVENTORY_FILE).exists(),
            "inventory must not be published"
        );
        ensure!(
            std::fs::read_dir(&root)?.count() == 1,
            "staging must be removed"
        );
        std::fs::remove_dir_all(root)?;
        Ok(())
    }
}
