#![allow(dead_code)]

use crate::release_support as signing;

use anyhow::Result;
use sinan_protocol::release::{ReleaseArtifact, canonical_asset_name};
use std::path::{Path, PathBuf};

pub fn write(
    root: &Path,
    name: &str,
    version: &str,
    binary_name: &str,
    archive: &[u8],
    binary: &[u8],
    format: &str,
) -> Result<PathBuf> {
    let mut artifacts: Vec<(ReleaseArtifact, Vec<u8>)> = Vec::new();
    for arch in ["amd64", "arm64"] {
        let mut entry = signing::entry(name, version, binary_name, format, archive, binary);
        entry.arch = arch.into();
        entry.asset_name = canonical_asset_name(&entry)?;
        artifacts.push((entry, archive.to_vec()));
    }
    let proof = signing::signed_release(artifacts.clone());
    let release = root.join("artifacts/releases/agent-v0.3.0");
    std::fs::create_dir_all(&release)?;
    signing::install_proof(&release, &proof);
    std::fs::write(release.join("install.sh"), b"#!/bin/sh\nexit 0\n")?;
    for (entry, bytes) in artifacts {
        let directory = release.join(&entry.name).join(&entry.version);
        std::fs::create_dir_all(&directory)?;
        std::fs::write(directory.join(&entry.arch), bytes)?;
    }
    Ok(release.join(name).join(version))
}

pub fn archive(binary_name: &str, binary: &[u8]) -> Result<Vec<u8>> {
    let mut archive = tar::Builder::new(flate2::write::GzEncoder::new(
        Vec::new(),
        flate2::Compression::default(),
    ));
    let mut header = tar::Header::new_gnu();
    header.set_size(binary.len() as u64);
    header.set_mode(0o755);
    header.set_cksum();
    archive.append_data(&mut header, binary_name, binary)?;
    Ok(archive.into_inner()?.finish()?)
}
