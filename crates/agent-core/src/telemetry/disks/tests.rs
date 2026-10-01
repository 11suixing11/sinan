use super::*;

#[derive(Clone, Debug, PartialEq)]
struct Sample {
    name: &'static str,
    path: &'static str,
    capacity: u64,
    available: u64,
}

fn mount(
    name: &'static str,
    path: &'static str,
    file_system: &str,
    capacity: u64,
    available: u64,
) -> (Mount, Sample) {
    (
        Mount {
            identity: Identity::Mount(path.into()),
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

#[test]
fn volume_labels_do_not_merge_distinct_mount_identities() {
    let samples = select(vec![
        mount("Data", "C:\\", "NTFS", 1000, 200),
        mount("Data", "D:\\", "NTFS", 2000, 1500),
        mount("", "E:\\", "NTFS", 3000, 2500),
        mount("", "F:\\", "NTFS", 4000, 3000),
    ]);
    assert_eq!(samples.len(), 4);
    assert_eq!(sample_totals(&samples), Some((10000, 2800)));
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
