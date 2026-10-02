use super::*;

#[cfg(unix)]
#[derive(Clone, Debug, PartialEq)]
struct Sample {
    name: &'static str,
    path: &'static str,
    capacity: u64,
    available: u64,
}

#[cfg(unix)]
fn mount(
    name: &'static str,
    path: &'static str,
    file_system: &str,
    capacity: u64,
    available: u64,
) -> (Mount, Sample) {
    (
        Mount {
            identity: unix_identity(name.as_ref(), file_system.as_ref(), Path::new(path), None),
            source: name.into(),
            path: path.into(),
            file_system: file_system.into(),
            is_file: false,
        },
        Sample {
            name,
            path,
            capacity,
            available,
        },
    )
}

#[cfg(target_os = "linux")]
fn device_mount(
    name: &'static str,
    path: &'static str,
    file_system: &str,
    device: u64,
    capacity: u64,
    available: u64,
) -> (Mount, Sample) {
    let mut entry = mount(name, path, file_system, capacity, available);
    entry.0.identity = unix_identity(
        name.as_ref(),
        file_system.as_ref(),
        Path::new(path),
        Some(device),
    );
    entry
}

#[cfg(target_os = "linux")]
const THIN_DEVICE: &str = "/dev/mapper/docker-253:0-12345-0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[cfg(unix)]
fn sample_totals(samples: &[Sample]) -> Option<(u64, u64)> {
    capacity_totals(samples.iter().map(|s| (s.capacity, s.available)))
}

#[cfg(target_os = "linux")]
#[test]
fn host_capacity_excludes_nested_overlays_and_keeps_distinct_partitions() {
    let samples = select(vec![
        mount(
            "overlay",
            "/var/lib/docker/overlay2/a/merged",
            "overlay",
            1000,
            200,
        ),
        mount("/dev/vda3", "/", "ext4", 1000, 200),
        mount("/dev/vda2", "/boot", "ext4", 100, 60),
        mount("/dev/vda1", "/boot/efi", "vfat", 20, 15),
        mount(
            "overlay",
            "/var/lib/docker/overlay2/b/merged",
            "overlay",
            1000,
            200,
        ),
        mount(
            "rootless",
            "/srv/containers/merged",
            "fuse.overlayfs",
            1000,
            200,
        ),
        mount("legacy", "/srv/legacy/merged", "overlayfs", 1000, 200),
    ]);
    assert_eq!(
        samples.iter().map(|s| s.path).collect::<Vec<_>>(),
        ["/", "/boot", "/boot/efi"]
    );
    assert_eq!(sample_totals(&samples), Some((1120, 845)));
}

#[cfg(target_os = "linux")]
#[test]
fn container_retains_overlay_root_but_not_file_mounts() {
    let mut mounts = vec![mount("overlay", "/", "overlay", 1000, 200)];
    for path in ["/etc/hosts", "/etc/hostname", "/etc/resolv.conf"] {
        let mut entry = mount("/dev/vda3", path, "ext4", 1000, 200);
        entry.0.is_file = true;
        mounts.push(entry);
    }
    let samples = select(mounts);
    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].path, "/");
    assert_eq!(sample_totals(&samples), Some((1000, 800)));
}

#[cfg(target_os = "linux")]
#[test]
fn container_driver_views_do_not_multiply_host_capacity() {
    let mut mounts = vec![
        device_mount("/dev/vda1", "/", "ext4", 1, 1000, 200),
        device_mount("/dev/vdb1", "/var/lib/docker", "ext4", 2, 2000, 500),
        device_mount(
            "/dev/mapper/docker-data",
            "/docker-data",
            "ext4",
            3,
            3000,
            1500,
        ),
        device_mount(
            "/dev/mapper/docker-pool",
            "/docker-pool",
            "xfs",
            4,
            4000,
            2000,
        ),
        device_mount("/dev/vdc1", "/data/docker-backups", "ext4", 5, 5000, 2500),
        device_mount("/dev/example-btrfs", "/srv/btrfs", "btrfs", 6, 6000, 3000),
    ];
    for (source, path, filesystem) in [
        (
            "fuse-overlayfs",
            "/srv/rootless/merged",
            "fuse.fuse-overlayfs",
        ),
        (THIN_DEVICE, "/custom/container", "ext4"),
        ("none", "/custom/aufs/mnt/id", "aufs"),
    ] {
        mounts.push(device_mount(source, path, filesystem, 100, 1000, 200));
    }
    for path in [
        "/custom/overlay2/id/merged",
        "/custom/overlay/id/merged",
        "/var/lib/docker/volumes/id/_data",
        "/home/example/docker/containers/id",
    ] {
        // Bind mounts keep their backing filesystem identity and capacity.
        mounts.push(device_mount("/dev/root-alias", path, "ext4", 1, 1000, 200));
    }
    // Btrfs snapshots have distinct st_dev values, but share their source device.
    mounts.push(device_mount(
        "/dev/example-btrfs",
        "/custom/io.containerd.snapshotter.v1.btrfs/snapshots/1/fs",
        "btrfs",
        7,
        6000,
        3000,
    ));
    let samples = select(mounts);
    assert_eq!(samples.len(), 6);
    assert_eq!(sample_totals(&samples), Some((21000, 11300)));
    assert!(
        samples
            .iter()
            .any(|sample| sample.path == "/var/lib/docker")
    );
}

#[cfg(target_os = "linux")]
#[test]
fn ordinary_partitions_remain_visible_under_container_like_directories() {
    let paths = [
        "/mnt/overlay/backups",
        "/media/overlay2/archive",
        "/srv/aufs/data",
        "/var/lib/docker/volumes/archive/_data",
        "/custom/devicemapper/mnt/archive",
        "/home/example/docker/containers/storage",
        "/custom/io.containerd.snapshotter.v1.btrfs/physical-disk",
    ];
    let mounts = paths
        .iter()
        .enumerate()
        .map(|(index, path)| {
            // Independent physical devices must not be rejected by directory names.
            device_mount("/dev/disk-alias", path, "ext4", index as u64 + 1, 1000, 400)
        })
        .collect();
    let samples = select(mounts);
    assert_eq!(samples.len(), paths.len());
    assert_eq!(sample_totals(&samples), Some((7000, 4200)));
    // A loop-backed filesystem is also retained without positive driver evidence.
    let samples = select(vec![device_mount(
        "/dev/loop0",
        "/custom/devicemapper/mnt/id",
        "ext4",
        8,
        2000,
        500,
    )]);
    assert_eq!(sample_totals(&samples), Some((2000, 1500)));
}

#[cfg(target_os = "linux")]
#[test]
fn container_root_capacity_is_retained_for_each_storage_driver() {
    for (source, filesystem) in [
        ("fuse-overlayfs", "fuse.fuse-overlayfs"),
        (THIN_DEVICE, "ext4"),
        ("none", "aufs"),
        ("overlay", "overlay"),
        ("overlay", "overlayfs"),
        ("fuse-overlayfs", "fuse.overlayfs"),
    ] {
        let samples = select(vec![mount(source, "/", filesystem, 1000, 200)]);
        assert_eq!(samples.len(), 1);
        assert_eq!(sample_totals(&samples), Some((1000, 800)));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn container_device_filter_preserves_non_container_volume_names() {
    for source in [
        "/dev/mapper/docker-data",
        "/dev/mapper/docker-pool",
        "/dev/mapper/docker-253:0-data-container",
        "/dev/mapper/docker-253:0-12345-",
        "/dev/mapper/docker-:0-12345-container",
        "/dev/mapper/docker-253:0-12345-archive",
        "/dev/mapper/docker-253:0-12345-012345",
        "/dev/mapper/docker-253:0-12345-base-init",
        "/dev/vda1",
    ] {
        assert!(!container_device(source.as_ref()), "{source}");
    }
    assert!(container_device(THIN_DEVICE.as_ref()));
    assert!(container_device(format!("{THIN_DEVICE}-init").as_ref()));
    assert!(container_device(
        "/dev/mapper/docker-253:0-12345-base".as_ref()
    ));
    for source in [
        THIN_DEVICE.replace("253:0", "253:"),
        THIN_DEVICE.replace("12345", "inode"),
        THIN_DEVICE.replace("abcdef", "ghijkl"),
        format!("{THIN_DEVICE}-backup"),
    ] {
        assert!(!container_device(source.as_ref()), "{source}");
    }
}

#[cfg(unix)]
#[test]
fn filesystem_identity_deduplicates_bind_mounts_and_device_aliases() {
    let mut root = mount("/dev/root", "/", "ext4", 1000, 200);
    root.0.identity = Identity::Device(1);
    let mut bind = mount("/dev/vda3", "/srv/bind", "ext4", 1000, 200);
    bind.0.identity = Identity::Device(1);
    let mut data = mount("/dev/vdb1", "/data", "ext4", 1000, 500);
    data.0.identity = Identity::Device(2);
    let samples = select(vec![bind, data, root]);
    assert_eq!(
        samples.iter().map(|s| s.path).collect::<Vec<_>>(),
        ["/", "/data"]
    );
    // Equal sizes do not make independent filesystems duplicates.
    assert_eq!(sample_totals(&samples), Some((2000, 1300)));
}

#[cfg(unix)]
#[test]
fn unavailable_metadata_uses_device_name_as_fallback() {
    let mut root = mount("/dev/vda3", "/", "ext4", 1000, 200);
    root.0.identity = unix_identity("/dev/vda3".as_ref(), "ext4".as_ref(), &root.0.path, None);
    let mut alias = mount("/dev/vda3", "/srv/bind", "ext4", 1000, 200);
    alias.0.identity = unix_identity("/dev/vda3".as_ref(), "ext4".as_ref(), &alias.0.path, None);
    assert_eq!(select(vec![alias, root]).len(), 1);
}

#[cfg(unix)]
#[test]
fn btrfs_subvolumes_keep_source_device_deduplication() {
    let mut root = mount("/dev/example-btrfs", "/", "btrfs", 1000, 200);
    let mut home = mount("/dev/example-btrfs", "/home", "btrfs", 1000, 200);
    for (entry, device) in [(&mut root, 1), (&mut home, 2)] {
        entry.0.identity = unix_identity(
            entry.1.name.as_ref(),
            &entry.0.file_system,
            &entry.0.path,
            Some(device),
        );
    }
    let samples = select(vec![home, root]);
    assert_eq!(samples.len(), 1);
    assert_eq!(sample_totals(&samples), Some((1000, 800)));
}

#[cfg(unix)]
#[test]
fn unavailable_names_keep_distinct_mount_fallbacks() {
    let samples = select(vec![
        mount("", "/data/a", "ext4", 1000, 200),
        mount("", "/data/b", "ext4", 2000, 1500),
        mount("", "/data/c", "ext4", 3000, 2500),
        mount("", "/data/d", "ext4", 4000, 3000),
    ]);
    assert_eq!(samples.len(), 4);
    assert_eq!(sample_totals(&samples), Some((10000, 2800)));
}

#[test]
fn legacy_volume_labels_count_aliases_once_and_keep_first_sample() {
    let data = std::ffi::OsStr::new("Data");
    let backup = std::ffi::OsStr::new("Backup");
    // One volume at a drive root and a directory mount must not double capacity.
    assert_eq!(
        named_capacity_totals([(data, 1000, 200), (data, 1000, 100), (backup, 2000, 1500)]),
        Some((3000, 1300))
    );
    assert_eq!(
        named_capacity_totals([(data, 0, 0), (data, 1000, 200)]),
        None
    );
    assert_eq!(named_capacity_totals([(data, 100, 101)]), None);
    assert_eq!(
        named_capacity_totals([(data, u64::MAX, 0), (backup, 1, 0)]),
        None
    );
    assert_eq!(named_capacity_totals([(data, 1000, 1000)]), Some((1000, 0)));
}

#[test]
fn missing_invalid_or_overflowing_capacity_stays_unknown() {
    for capacities in [
        vec![],
        vec![(0, 0)],
        vec![(1000, 200), (0, 0)],
        vec![(100, 101)],
        vec![(u64::MAX, 0), (1, 0)],
    ] {
        assert_eq!(capacity_totals(capacities), None);
    }
    assert_eq!(capacity_totals([(1000, 1000)]), Some((1000, 0)));
    assert_eq!(capacity_totals([(1000, 0)]), Some((1000, 1000)));
}

#[cfg(unix)]
#[test]
fn collector_totals_match_reported_filesystems_after_refresh() {
    let mut collector = crate::telemetry::Collector::new();
    let metrics = collector.metrics();
    let expected = metrics.disks.iter().try_fold((0_u64, 0_u64), |sum, disk| {
        Some((
            sum.0.checked_add(disk.total_bytes?)?,
            sum.1.checked_add(disk.used_bytes?)?,
        ))
    });
    let expected = expected.filter(|(total, _)| *total > 0);
    assert_eq!(collector.static_info().disk_total, expected.map(|v| v.0));
    assert_eq!(metrics.disk_used, expected.map(|v| v.1));
    #[cfg(target_os = "linux")]
    for disk in &metrics.disks {
        if let Ok(metadata) = std::fs::metadata(&disk.mount_point) {
            assert!(!metadata.is_file());
        }
    }
}
