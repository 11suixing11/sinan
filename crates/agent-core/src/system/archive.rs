use crate::artifacts::safe_component;
use anyhow::{Context, Result, ensure};
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::Read,
    path::Path,
};

const MAX_UNPACKED: u64 = 256 * 1024 * 1024;

fn mode(path: &Path, mode: u32) -> Result<()> {
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    #[cfg(windows)]
    let _ = (path, mode);
    Ok(())
}
fn sync_directory(path: &Path) -> Result<()> {
    #[cfg(unix)]
    File::open(path)?.sync_all()?;
    #[cfg(windows)]
    let _ = path;
    Ok(())
}

pub(super) fn install(
    archive: &Path,
    directory: &Path,
    binary_name: &str,
    extras: &[String],
) -> Result<()> {
    let expected: BTreeSet<&str> = std::iter::once(binary_name)
        .chain(extras.iter().map(String::as_str))
        .collect();
    ensure!(
        expected.len() == extras.len() + 1
            && expected.len() <= 8
            && expected.iter().all(|name| safe_component(name)),
        "invalid artifact file names"
    );
    ensure!(
        fs::symlink_metadata(directory).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound),
        "artifact version already exists"
    );
    let parent = directory.parent().context("artifact has no parent")?;
    fs::create_dir_all(parent)?;
    let staging = parent.join(format!(".unpack-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&staging)?;
    mode(&staging, 0o700)?;
    let result = (|| -> Result<()> {
        let decoder =
            flate2::read::MultiGzDecoder::new(File::open(archive)?).take(MAX_UNPACKED + 1);
        let mut archive = tar::Archive::new(decoder);
        let mut found = BTreeSet::new();
        for entry in archive.entries()?.raw(true) {
            let mut entry = entry?;
            ensure!(
                entry.header().entry_type().is_file(),
                "archive contains a non-ordinary file"
            );
            let name = std::str::from_utf8(entry.path_bytes().as_ref())?.to_owned();
            ensure!(
                expected.contains(name.as_str()) && found.insert(name.clone()),
                "archive file name does not match the expected file set"
            );
            ensure!(
                entry.size() <= MAX_UNPACKED,
                "artifact exceeds unpacked size limit"
            );
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            options.mode(0o600);
            let path = staging.join(name);
            let mut file = options.open(&path)?;
            let size = entry.size();
            ensure!(
                std::io::copy(&mut entry, &mut file)? == size,
                "archive file was truncated"
            );
            mode(&path, 0o755)?;
            file.sync_all()?;
        }
        ensure!(
            found.iter().map(String::as_str).collect::<BTreeSet<_>>() == expected,
            "archive is missing expected files"
        );
        let mut decoder = archive.into_inner();
        let mut tail = [0u8; 8192];
        loop {
            let count = decoder.read(&mut tail)?;
            if count == 0 {
                break;
            }
            ensure!(
                tail[..count].iter().all(|v| *v == 0),
                "archive contains trailing data"
            );
        }
        ensure!(decoder.limit() > 0, "artifact exceeds unpacked size limit");
        mode(&staging, 0o755)?;
        sync_directory(&staging)?;
        ensure!(
            fs::symlink_metadata(directory)
                .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound),
            "artifact version appeared during installation"
        );
        fs::rename(&staging, directory)?;
        sync_directory(parent)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(staging);
    }
    result
}
