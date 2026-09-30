use super::{MARKER, ensure_ordinary_directory_if_present};
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use sinan_adapter_sdk::Descriptor;
use sinan_protocol::release::{
    MAX_CHECKSUMS_BYTES, MAX_METADATA_BYTES, MAX_SIGNATURE_BYTES, ReleaseError, ReleaseProof,
    TrustedKeys, VerifiedArtifact, VerifiedRelease, verify_release,
};
use std::path::Path;
use tokio::io::AsyncReadExt;

pub(super) const MAX_PROOF: usize =
    2 * (MAX_METADATA_BYTES + MAX_CHECKSUMS_BYTES + MAX_SIGNATURE_BYTES);

pub(super) async fn ordinary_bytes(path: &Path, maximum: usize) -> Result<Vec<u8>> {
    let metadata = tokio::fs::symlink_metadata(path).await?;
    ensure!(
        metadata.is_file() && metadata.len() <= maximum as u64,
        "proof must be an ordinary bounded file"
    );
    let mut bytes = Vec::new();
    tokio::fs::File::open(path)
        .await?
        .take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .await?;
    ensure!(bytes.len() <= maximum, "proof exceeds size limit");
    Ok(bytes)
}

pub(super) async fn read_proof(directory: &Path) -> Result<ReleaseProof> {
    ensure_ordinary_directory_if_present(directory).await?;
    ensure_ordinary_directory_if_present(
        directory
            .parent()
            .context("artifact directory has no parent")?,
    )
    .await?;
    let metadata = directory.join("release.json");
    if tokio::fs::symlink_metadata(&metadata).await.is_ok() {
        return Ok(ReleaseProof {
            metadata_json: String::from_utf8(ordinary_bytes(&metadata, MAX_METADATA_BYTES).await?)?,
            checksums: String::from_utf8(
                ordinary_bytes(&directory.join("SHA256SUMS"), MAX_CHECKSUMS_BYTES).await?,
            )?,
            signature: String::from_utf8(
                ordinary_bytes(&directory.join("SHA256SUMS.minisig"), MAX_SIGNATURE_BYTES).await?,
            )?,
        });
    }
    let marker: super::InstalledArtifact = serde_json::from_slice(
        &ordinary_bytes(&directory.join(MARKER), MAX_PROOF)
            .await
            .context("installed artifact has no signed proof")?,
    )?;
    marker
        .proof
        .context("installed artifact has no signed proof")
}

pub(super) async fn verify_file(binary: &Path, verified: &VerifiedArtifact) -> Result<()> {
    let metadata = tokio::fs::symlink_metadata(binary).await?;
    ensure!(
        metadata.is_file() && metadata.len() == verified.metadata().binary_size,
        "installed artifact is not an ordinary file of its signed size"
    );
    let mut file = tokio::fs::File::open(binary).await?;
    let mut hash = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024];
    let mut length = 0_u64;
    loop {
        let count = file.read(&mut buffer).await?;
        if count == 0 {
            break;
        }
        length += count as u64;
        ensure!(
            length <= verified.metadata().binary_size,
            "installed artifact grew beyond signed size"
        );
        hash.update(&buffer[..count]);
    }
    ensure!(
        length == verified.metadata().binary_size
            && format!("{:x}", hash.finalize()) == verified.metadata().binary_sha256,
        "installed artifact binary differs from signed release"
    );
    let directory = binary.parent().context("binary has no parent")?;
    for (name, expected) in &verified.metadata().auxiliary_files {
        let bytes = ordinary_bytes(&directory.join(name), expected.size as usize).await?;
        ensure!(
            bytes.len() as u64 == expected.size
                && format!("{:x}", Sha256::digest(&bytes)) == expected.sha256,
            "auxiliary artifact differs from signed release"
        );
    }
    Ok(())
}

pub(super) fn signed_release(proof: &ReleaseProof, keys: &TrustedKeys) -> Result<VerifiedRelease> {
    let release = verify_release(proof, keys)?;
    ensure!(
        release.metadata().protocol_min <= sinan_protocol::PROTOCOL_MAX
            && release.metadata().protocol_max >= sinan_protocol::PROTOCOL_MIN,
        "signed release protocol range is incompatible with this Agent"
    );
    Ok(release)
}

pub(super) fn signed_artifact(
    proof: &ReleaseProof,
    descriptor: &Descriptor,
    version: &str,
    keys: &TrustedKeys,
) -> Result<VerifiedArtifact> {
    let release = signed_release(proof, keys)?;
    let verified = runtime_artifact(&release, &descriptor.plugin_name, version)?;
    ensure!(
        verified.metadata().binary_name == descriptor.binary_name
            && verified.metadata().format == "tar.gz"
            && verified
                .metadata()
                .auxiliary_files
                .keys()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>()
                == descriptor
                    .auxiliary_files
                    .iter()
                    .cloned()
                    .collect::<std::collections::BTreeSet<_>>(),
        "signed artifact format or executable name differs from adapter"
    );
    Ok(verified)
}

pub(super) fn runtime_artifact(
    release: &VerifiedRelease,
    name: &str,
    version: &str,
) -> Result<VerifiedArtifact> {
    let target = crate::system::platform::runtime_target()?;
    runtime_artifact_for_target(
        release,
        name,
        version,
        &target,
        sinan_protocol::release::native_arch()?,
    )
}

fn runtime_artifact_for_target(
    release: &VerifiedRelease,
    name: &str,
    version: &str,
    target: &str,
    arch: &str,
) -> Result<VerifiedArtifact> {
    let mut targets = vec![target.to_owned(), arch.to_owned()];
    // Existing signed static runtime caches remain executable on GNU hosts.
    if target == format!("linux-gnu-{arch}") {
        targets.push(format!("linux-musl-{arch}"));
    }
    for target in targets {
        match release.artifact(name, version, &target) {
            Ok(artifact) => return Ok(artifact),
            Err(ReleaseError::MissingArtifact) => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err(ReleaseError::MissingArtifact.into())
}

pub(crate) async fn verify_expected(
    binary: &Path,
    descriptor: &Descriptor,
    version: &str,
    keys: &TrustedKeys,
) -> Result<()> {
    let directory = binary
        .parent()
        .context("artifact binary has no directory")?;
    ensure!(
        directory.file_name().and_then(|name| name.to_str()) == Some(version)
            && binary.file_name().and_then(|name| name.to_str())
                == Some(descriptor.binary_name.as_str()),
        "installed artifact path differs from requested identity"
    );
    let proof = read_proof(directory).await?;
    let verified = signed_artifact(&proof, descriptor, version, keys)?;
    verify_file(binary, &verified).await
}

pub(super) async fn verify_binary_with_keys(
    binary: &Path,
    expected_name: &str,
    expected_format: &str,
    keys: &TrustedKeys,
) -> Result<()> {
    ensure!(
        binary.is_absolute(),
        "artifact verification requires an absolute path"
    );
    let directory = binary.parent().context("binary has no parent")?;
    let directory_metadata = tokio::fs::symlink_metadata(directory).await?;
    let resolved = if directory_metadata.file_type().is_symlink() {
        ensure!(
            directory.file_name().is_some_and(|name| name == "current"),
            "uncontrolled artifact directory symlink"
        );
        let resolved = tokio::fs::canonicalize(directory).await?;
        ensure!(
            resolved.parent()
                == Some(
                    tokio::fs::canonicalize(directory.parent().context("current has no parent")?)
                        .await?
                        .as_path()
                ),
            "current artifact link escapes its installation directory"
        );
        resolved
    } else {
        ensure!(
            directory_metadata.is_dir(),
            "artifact version is not an ordinary directory"
        );
        directory.to_path_buf()
    };
    let version = resolved
        .file_name()
        .and_then(|name| name.to_str())
        .context("invalid artifact version path")?;
    let name = binary
        .file_name()
        .and_then(|name| name.to_str())
        .context("invalid binary path")?;
    let proof = read_proof(&resolved).await?;
    let release = signed_release(&proof, keys)?;
    let verified = if expected_name == "agent" {
        release.native_artifact(expected_name, version)?
    } else {
        runtime_artifact(&release, expected_name, version)?
    };
    ensure!(
        verified.metadata().binary_name == name && verified.metadata().format == expected_format,
        "installed artifact role, format, or executable name differs from requested identity"
    );
    verify_file(&resolved.join(name), &verified).await
}

/// Verifies a cached executable offline without reading device identity or configuration.
pub async fn verify_installed_binary(
    binary: &Path,
    expected_name: &str,
    expected_format: &str,
) -> Result<()> {
    verify_binary_with_keys(
        binary,
        expected_name,
        expected_format,
        &TrustedKeys::compiled()?,
    )
    .await
}

/// Checks a release proof using this Agent's compiled trust roots, without network access.
pub async fn verify_release_directory(directory: &Path) -> Result<()> {
    ensure_ordinary_directory_if_present(directory).await?;
    ensure_ordinary_directory_if_present(
        directory
            .parent()
            .context("release directory has no parent")?,
    )
    .await?;
    let proof = ReleaseProof {
        metadata_json: String::from_utf8(
            ordinary_bytes(&directory.join("release.json"), MAX_METADATA_BYTES).await?,
        )?,
        checksums: String::from_utf8(
            ordinary_bytes(&directory.join("SHA256SUMS"), MAX_CHECKSUMS_BYTES).await?,
        )?,
        signature: String::from_utf8(
            ordinary_bytes(&directory.join("SHA256SUMS.minisig"), MAX_SIGNATURE_BYTES).await?,
        )?,
    };
    signed_release(&proof, &TrustedKeys::compiled()?)?;
    Ok(())
}

#[cfg(test)]
mod platform_tests {
    use super::*;
    use crate::release_test_support as fixture;

    fn release(targets: &[&str]) -> VerifiedRelease {
        let entries = targets
            .iter()
            .map(|target| {
                let binary = target.as_bytes();
                let mut entry = fixture::entry("runtime", "1", "runtime", "tar.gz", binary, binary);
                entry.arch = (*target).into();
                entry.asset_name = sinan_protocol::release::canonical_asset_name(&entry).unwrap();
                (entry, binary.to_vec())
            })
            .collect();
        signed_release(&fixture::signed_release(entries), &fixture::trusted_keys()).unwrap()
    }

    #[test]
    fn runtime_proof_uses_host_abi_and_rejects_foreign_targets() {
        let both = release(&["linux-gnu-amd64", "linux-musl-amd64"]);
        for target in ["linux-gnu-amd64", "linux-musl-amd64"] {
            let selected =
                runtime_artifact_for_target(&both, "runtime", "1", target, "amd64").unwrap();
            assert_eq!(selected.metadata().arch, target);
            selected.verify_binary(target.as_bytes()).unwrap();
        }
        let gnu = release(&["linux-gnu-amd64"]);
        assert!(
            runtime_artifact_for_target(&gnu, "runtime", "1", "linux-musl-amd64", "amd64").is_err()
        );
        assert!(
            runtime_artifact_for_target(&gnu, "runtime", "1", "linux-gnu-arm64", "arm64").is_err()
        );
        let static_only = release(&["linux-musl-amd64"]);
        assert_eq!(
            runtime_artifact_for_target(&static_only, "runtime", "1", "linux-gnu-amd64", "amd64")
                .unwrap()
                .metadata()
                .arch,
            "linux-musl-amd64"
        );
    }
}
