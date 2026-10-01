use super::*;

#[test]
fn whitelist_requires_explicit_no_upload_and_rejects_unbounded_or_host_options() {
    let base = [
        "--workspace",
        "/private/fixture",
        "--targets",
        "targets.json",
        "--target-digest",
        &"a".repeat(64),
        "--ip-version",
        "4",
        "--no-rank-upload",
    ]
    .map(str::to_owned)
    .to_vec();
    assert!(matches!(parse(base.clone()), Ok(Command::Run(_))));
    let mut missing = base.clone();
    missing.pop();
    assert!(parse(missing).is_err());
    for flag in [
        "--allow-speedtest-staged",
        "--no-rootfs",
        "--speedtest",
        "--all",
        "--rank-upload",
        "--debug",
        "--command",
        "--no-rank-upload",
    ] {
        let mut args = base.clone();
        args.push(flag.into());
        assert!(parse(args).is_err(), "{flag}");
    }
    for (flag, value) in [
        ("--count", "100"),
        ("--count", "0"),
        ("--concurrency", "16"),
        ("--ip-version", "all"),
        ("--targets", "../target.json"),
        ("--targets", "/etc/passwd"),
        ("--workspace", "/"),
        ("--target-digest", "not-a-digest"),
    ] {
        let mut args = base.clone();
        if let Some(index) = args.iter().position(|item| item == flag) {
            args[index + 1] = value.into();
        } else {
            args.extend([flag.into(), value.into()]);
        }
        assert!(parse(args).is_err(), "{flag} {value}");
    }
    assert!(matches!(parse(["--help".into()]), Ok(Command::Help)));
    assert!(matches!(parse(["--version".into()]), Ok(Command::Version)));
}

#[test]
fn targets_validate_scope_and_never_guess_region_or_enable_arbitrary_urls() {
    let normal = Snapshot {
        schema: 1,
        targets: vec![target(1, "sample.example.test", 443)],
    };
    assert!(normal.valid());
    assert!(normal.targets[0].region.is_none());
    for host in [
        "https://example.test",
        "a/b",
        "a\nb",
        "--help",
        "a..test",
        "0.0.0.0",
        "::",
        "224.0.0.1",
        "ff02::1",
        "255.255.255.255",
    ] {
        let mut snapshot = normal.clone();
        snapshot.targets[0].target = host.into();
        assert!(!snapshot.valid(), "{host}");
    }
    let mut duplicate = normal.clone();
    duplicate.targets.push(duplicate.targets[0].clone());
    assert!(!duplicate.valid());
    let too_many = Snapshot {
        schema: 1,
        targets: (1..=9).map(|id| target(id, "127.0.0.1", 443)).collect(),
    };
    assert!(!too_many.valid());
    let mut wrong = normal.clone();
    wrong.targets[0].port = 0;
    assert!(!wrong.valid());
    wrong = normal;
    wrong.targets[0].region = Some("".into());
    assert!(!wrong.valid());
    assert!(
        serde_json::from_str::<Snapshot>(r#"{"schema":1,"targets":[],"command":"unexpected"}"#)
            .is_err()
    );
}

#[tokio::test]
async fn snapshot_limit_digest_and_existing_output_are_enforced_before_probing() {
    for bytes in [
        b"null".to_vec(),
        vec![b'x'; INPUT_LIMIT + 1],
        br#"{"schema":1,"targets":[]}"#.to_vec(),
    ] {
        let directory = Directory::new();
        directory.input(&bytes);
        assert!(
            Journal::open(&directory.options(&bytes, IpVersion::V4))
                .await
                .is_err()
        );
        assert!(!directory.path.join("result.json").exists());
        assert!(!directory.path.join("sections").exists());
    }
    let directory = Directory::new();
    let bytes = serde_json::to_vec(&Snapshot {
        schema: 1,
        targets: vec![target(1, "127.0.0.1", 443)],
    })
    .unwrap();
    directory.input(&bytes);
    let mut options = directory.options(&bytes, IpVersion::V4);
    options.targets_file = "../outside.json".into();
    assert!(Journal::open(&options).await.is_err());
    options.targets_file = "targets.json".into();
    options.target_digest = "0".repeat(64);
    assert!(Journal::open(&options).await.is_err());
    options.target_digest = format!("{:x}", Sha256::digest(&bytes));
    std::fs::write(
        directory.path.join("result.json"),
        b"saved historical report",
    )
    .unwrap();
    assert!(Journal::open(&options).await.is_err());
    assert_eq!(
        std::fs::read(directory.path.join("result.json")).unwrap(),
        b"saved historical report"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn public_directories_symlinks_and_hardlinks_are_rejected() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let directory = Directory::new();
    let bytes = serde_json::to_vec(&Snapshot {
        schema: 1,
        targets: vec![target(1, "127.0.0.1", 443)],
    })
    .unwrap();
    directory.input(&bytes);
    let options = directory.options(&bytes, IpVersion::V4);
    std::fs::set_permissions(&directory.path, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(Journal::open(&options).await.is_err());
    std::fs::set_permissions(&directory.path, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::hard_link(
        directory.path.join("targets.json"),
        directory.path.join("linked.json"),
    )
    .unwrap();
    assert!(Journal::open(&options).await.is_err());
    std::fs::remove_file(directory.path.join("linked.json")).unwrap();
    std::fs::rename(
        directory.path.join("targets.json"),
        directory.path.join("frozen.json"),
    )
    .unwrap();
    symlink(
        directory.path.join("frozen.json"),
        directory.path.join("targets.json"),
    )
    .unwrap();
    assert!(Journal::open(&options).await.is_err());
    let link = Directory::new();
    symlink(&directory.path, link.path.join("workspace")).unwrap();
    let mut options = options;
    options.workspace = link.path.join("workspace");
    assert!(Journal::open(&options).await.is_err());
}
