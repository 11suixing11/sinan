use std::{
    collections::BTreeSet,
    ffi::OsString,
    path::{Path, PathBuf},
};
use sysinfo::{Disk, Disks};

#[derive(Eq, Ord, PartialEq, PartialOrd)]
enum Identity {
    #[cfg(unix)]
    Device(u64),
    #[cfg(unix)]
    Name(OsString),
    Mount(PathBuf),
}

struct Mount {
    identity: Identity,
    path: PathBuf,
    file_system: OsString,
    is_file: bool,
}

impl Mount {
    fn from_disk(disk: &Disk) -> Self {
        let path = disk.mount_point().to_path_buf();
        let metadata = std::fs::metadata(&path).ok();
        #[cfg(unix)]
        let identity = {
            use std::os::unix::fs::MetadataExt;
            unix_identity(
                disk.name(),
                disk.file_system(),
                &path,
                metadata.as_ref().map(MetadataExt::dev),
            )
        };
        // Windows disk names are volume labels, which need not be unique.
        #[cfg(not(unix))]
        let identity = Identity::Mount(path.clone());
        Self {
            identity,
            path,
            file_system: disk.file_system().to_owned(),
            is_file: metadata.is_some_and(|metadata| metadata.is_file()),
        }
    }
}

#[cfg(unix)]
fn unix_identity(
    name: &std::ffi::OsStr,
    file_system: &std::ffi::OsStr,
    path: &Path,
    device: Option<u64>,
) -> Identity {
    // Btrfs subvolumes share capacity even when st_dev differs. Preserve
    // source-device deduplication, resolving aliases when accessible.
    if file_system == "btrfs" && !name.is_empty() {
        return Identity::Name(
            std::fs::canonicalize(name)
                .map(PathBuf::into_os_string)
                .unwrap_or_else(|_| name.to_owned()),
        );
    }
    match device {
        Some(device) => Identity::Device(device),
        None if !name.is_empty() => Identity::Name(name.to_owned()),
        None => Identity::Mount(path.to_path_buf()),
    }
}

pub(super) fn refresh() -> Vec<Disk> {
    // FreeBSD getmntinfo and libgeom use process-global storage. Keep the lock
    // until sysinfo has copied every mount and completed its I/O snapshot.
    #[cfg(target_os = "freebsd")]
    static DISKS: std::sync::Mutex<()> = std::sync::Mutex::new(());
    #[cfg(target_os = "freebsd")]
    let _guard = DISKS.lock().unwrap_or_else(|error| error.into_inner());
    let disks = Vec::from(Disks::new_with_refreshed_list());
    select(
        disks
            .into_iter()
            .map(|disk| (Mount::from_disk(&disk), disk))
            .collect(),
    )
}

fn select<T>(mut mounts: Vec<(Mount, T)>) -> Vec<T> {
    // Prefer the root and shallow mounts over aliases of the same filesystem.
    mounts.sort_by(|(a, _), (b, _)| {
        a.path
            .components()
            .count()
            .cmp(&b.path.components().count())
            .then_with(|| a.path.cmp(&b.path))
    });
    let mut seen = BTreeSet::new();
    mounts
        .into_iter()
        .filter(|(mount, _)| {
            if cfg!(target_os = "linux") {
                // Container overlay views reuse backing storage. Keep an overlay
                // root when running inside a container, but omit nested views.
                let overlay = matches!(
                    mount.file_system.to_str(),
                    Some("overlay" | "overlayfs" | "fuse.overlayfs")
                );
                if (overlay && mount.path != Path::new("/")) || mount.is_file {
                    return false;
                }
            }
            true
        })
        .filter_map(|(mount, disk)| seen.insert(mount.identity).then_some(disk))
        .collect()
}

pub(super) fn totals(disks: &[Disk]) -> Option<(u64, u64)> {
    capacity_totals(
        disks
            .iter()
            .map(|disk| (disk.total_space(), disk.available_space())),
    )
}

fn capacity_totals(capacities: impl IntoIterator<Item = (u64, u64)>) -> Option<(u64, u64)> {
    let mut total = 0_u64;
    let mut used = 0_u64;
    for (capacity, available) in capacities {
        super::positive(capacity)?;
        total = total.checked_add(capacity)?;
        used = used.checked_add(capacity.checked_sub(available)?)?;
    }
    super::positive(total).map(|total| (total, used))
}

#[cfg(test)]
mod tests;
