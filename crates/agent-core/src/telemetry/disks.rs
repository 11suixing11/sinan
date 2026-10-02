#[cfg(unix)]
use std::{
    collections::BTreeSet,
    ffi::OsString,
    path::{Path, PathBuf},
};
use sysinfo::{Disk, Disks};

#[cfg(unix)]
#[derive(Eq, Ord, PartialEq, PartialOrd)]
enum Identity {
    Device(u64),
    Name(OsString),
    Mount(PathBuf),
}

#[cfg(unix)]
struct Mount {
    identity: Identity,
    source: OsString,
    path: PathBuf,
    file_system: OsString,
    is_file: bool,
}

#[cfg(unix)]
impl Mount {
    fn from_disk(disk: &Disk) -> Self {
        let path = disk.mount_point().to_path_buf();
        let metadata = std::fs::metadata(&path).ok();
        let identity = {
            use std::os::unix::fs::MetadataExt;
            unix_identity(
                disk.name(),
                disk.file_system(),
                &path,
                metadata.as_ref().map(MetadataExt::dev),
            )
        };
        Self {
            identity,
            source: disk.name().to_owned(),
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
    #[cfg(unix)]
    {
        select(
            disks
                .into_iter()
                .map(|disk| (Mount::from_disk(&disk), disk))
                .collect(),
        )
    }
    // Keep the existing Windows inventory until stable volume IDs are available.
    #[cfg(not(unix))]
    {
        disks
    }
}

#[cfg(unix)]
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
                // Container union views reuse backing storage. Keep their
                // root when running inside a container, but omit nested views.
                let overlay = matches!(
                    mount.file_system.to_str(),
                    Some(
                        "overlay" | "overlayfs" | "fuse.overlayfs" | "fuse.fuse-overlayfs" | "aufs"
                    )
                );
                if mount.is_file
                    || (mount.path != Path::new("/")
                        && (overlay || container_device(&mount.source)))
                {
                    return false;
                }
            }
            true
        })
        .filter_map(|(mount, disk)| seen.insert(mount.identity).then_some(disk))
        .collect()
}

#[cfg(unix)]
fn container_device(source: &std::ffi::OsStr) -> bool {
    let Some(name) = source
        .to_str()
        .and_then(|source| source.strip_prefix("/dev/mapper/docker-"))
    else {
        return false;
    };
    let Some((device, remaining)) = name.split_once('-') else {
        return false;
    };
    let Some((major, minor)) = device.split_once(':') else {
        return false;
    };
    let Some((inode, container)) = remaining.split_once('-') else {
        return false;
    };
    // Thin devices use docker-MAJOR:MINOR-INODE-LAYER, including the base layer.
    // Ordinary LVM volumes such as docker-data and docker-pool remain visible.
    let layer = container.strip_suffix("-init").unwrap_or(container);
    [major, minor, inode]
        .iter()
        .all(|value| !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
        && (container == "base"
            || (layer.len() == 64 && layer.bytes().all(|byte| byte.is_ascii_hexdigit())))
}

pub(super) fn totals(disks: &[Disk]) -> Option<(u64, u64)> {
    #[cfg(unix)]
    {
        capacity_totals(
            disks
                .iter()
                .map(|disk| (disk.total_space(), disk.available_space())),
        )
    }
    #[cfg(not(unix))]
    {
        named_capacity_totals(
            disks
                .iter()
                .map(|disk| (disk.name(), disk.total_space(), disk.available_space())),
        )
    }
}

// sysinfo exposes Windows volume labels and can enumerate multiple mount paths
// for the same volume. Preserve legacy deduplication instead of counting aliases
// as independent capacity. Distinct volumes with equal labels remain ambiguous.
#[cfg(any(not(unix), test))]
fn named_capacity_totals<'a>(
    capacities: impl IntoIterator<Item = (&'a std::ffi::OsStr, u64, u64)>,
) -> Option<(u64, u64)> {
    let mut seen = std::collections::BTreeSet::new();
    capacity_totals(
        capacities
            .into_iter()
            .filter_map(|(name, total, available)| seen.insert(name).then_some((total, available))),
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
