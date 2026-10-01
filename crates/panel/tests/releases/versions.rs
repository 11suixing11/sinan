use super::*;
use sinan_protocol::release::canonical_asset_name;
use std::sync::Mutex;

fn platform_bundle(version: &str, protocol: (u16, u16), targets: &[&str]) -> Result<Bundle> {
    let artifacts: Vec<_> = targets
        .iter()
        .map(|target| {
            let binary = format!("fixture Agent {version} {target}").into_bytes();
            let name = if target.starts_with("windows-") {
                "sinan-agent.exe"
            } else {
                "sinan-agent"
            };
            let mut entry = signing::entry("agent", version, name, "raw", &binary, &binary);
            entry.arch = (*target).into();
            entry.asset_name = canonical_asset_name(&entry)?;
            Ok((entry, binary))
        })
        .collect::<Result<_>>()?;
    let mut proof = signing::signed_release(artifacts.clone());
    let mut metadata: ReleaseMetadata = serde_json::from_str(&proof.metadata_json)?;
    metadata.tag = format!("agent-v{version}");
    metadata.protocol_min = protocol.0;
    metadata.protocol_max = protocol.1;
    proof.metadata_json = format!("{}\n", serde_json::to_string(&metadata)?);
    proof.checksums = proof
        .checksums
        .lines()
        .map(|line| {
            if line.ends_with("  release.json") {
                format!(
                    "{}  release.json\n",
                    signing::hash(proof.metadata_json.as_bytes())
                )
            } else {
                format!("{line}\n")
            }
        })
        .collect();
    proof.signature = signing::sign(proof.checksums.as_bytes());
    Ok(Bundle {
        tag: metadata.tag,
        proof,
        assets: artifacts
            .into_iter()
            .map(|(entry, bytes)| (entry.asset_name, bytes))
            .chain(std::iter::once((
                "install.sh".into(),
                b"#!/bin/sh\nexit 0\n".to_vec(),
            )))
            .collect(),
    })
}

#[tokio::test]
async fn versions_are_numeric_platform_compatible_and_include_signed_uncached_targets() -> Result<()>
{
    let fixture = Fixture::new()?;
    let downloads = Mutex::new(Vec::new());
    for (version, protocol, targets, imported) in [
        ("0.9.0", (1, 1), vec!["amd64", "arm64"], "arm64"),
        (
            "0.10.0",
            (1, 2),
            vec!["linux-musl-arm64"],
            "linux-musl-arm64",
        ),
        ("1.0.0", (1, 1), vec!["windows-amd64"], "windows-amd64"),
        ("2.0.0", (1, 1), vec!["macos-arm64"], "macos-arm64"),
        ("3.0.0", (1, 1), vec!["freebsd-amd64"], "freebsd-amd64"),
        ("9.0.0", (2, 2), vec!["arm64"], "arm64"),
        ("4.0.0-rc.1", (1, 1), vec!["arm64"], "arm64"),
    ] {
        let bundle = platform_bundle(version, protocol, &targets)?;
        targets::import_target(&fixture.state, &bundle, imported, &downloads).await?;
    }
    let versions = releases::agent_versions(&fixture.state, None, None).await?;
    assert_eq!(
        versions
            .iter()
            .map(|entry| entry.version.as_str())
            .collect::<Vec<_>>(),
        ["3.0.0", "2.0.0", "1.0.0", "0.10.0", "0.9.0"]
    );
    let first = versions.last().context("Linux fixture missing")?;
    assert_eq!(first.targets, ["amd64", "arm64"]);
    assert_eq!(first.cached_targets, ["arm64"]);
    assert_eq!(
        releases::select_agent(&fixture.state, None).await?,
        ("0.10.0".into(), "agent-v0.10.0".into())
    );
    let arm = releases::agent_versions(&fixture.state, Some("linux-gnu-arm64"), None).await?;
    assert_eq!(
        arm.iter()
            .map(|entry| entry.version.as_str())
            .collect::<Vec<_>>(),
        ["0.10.0", "0.9.0"]
    );
    assert_eq!(
        releases::select_agent_for_target(&fixture.state, None, Some("windows-amd64")).await?,
        ("1.0.0".into(), "agent-v1.0.0".into())
    );
    assert_eq!(
        releases::select_agent_for_target(&fixture.state, None, Some("freebsd-amd64")).await?,
        ("3.0.0".into(), "agent-v3.0.0".into())
    );
    assert!(matches!(
        releases::select_agent_for_target(&fixture.state, Some("0.10.0"), Some("windows-amd64"))
            .await,
        Err(ApiError::Conflict(_))
    ));
    assert!(matches!(
        releases::agent_versions(&fixture.state, Some("riscv64"), None).await,
        Err(ApiError::BadRequest(_))
    ));
    let prerelease =
        releases::agent_versions(&fixture.state, Some("linux-musl-arm64"), Some("4.0.0-rc.1"))
            .await?;
    assert_eq!(prerelease[0].version, "4.0.0-rc.1");
    assert_eq!(prerelease[0].tag, "agent-v4.0.0-rc.1");
    Ok(())
}

#[tokio::test]
async fn on_demand_cache_adds_only_the_requested_agent_and_reuses_verified_bytes() -> Result<()> {
    let fixture = Fixture::new()?;
    let bundle = targets::multi_arch_bundle()?;
    targets::import_target(&fixture.state, &bundle, "arm64", &Mutex::new(Vec::new())).await?;
    let before = snapshot(&fixture.release_root())?;
    let downloads = Mutex::new(Vec::new());
    releases::cache_agent_payload(&fixture.state, "0.3.0", "amd64", |tag, name, maximum| {
        assert_eq!(tag, bundle.tag);
        assert!(name.starts_with("agent-") && name.ends_with("amd64"));
        downloads.lock().unwrap().push(name.clone());
        let bytes = bundle.assets.get(&name).cloned().context("fixture missing");
        async move {
            let bytes = bytes?;
            anyhow::ensure!(bytes.len() <= maximum);
            Ok(bytes)
        }
    })
    .await?;
    assert_eq!(downloads.lock().unwrap().len(), 1);
    assert_eq!(releases::entries(&fixture.state).await?.len(), 3);
    assert!(
        !fixture
            .release_root()
            .join(&bundle.tag)
            .join("sing-box/1.14.2/amd64")
            .exists()
    );
    for (path, bytes) in before {
        if path.ends_with("inventory.json") {
            continue;
        }
        assert_eq!(std::fs::read(fixture.release_root().join(path))?, bytes);
    }
    releases::cache_agent_payload(&fixture.state, "0.3.0", "amd64", |_, _, _| async {
        bail!("verified cached Agent must not download again")
    })
    .await?;
    let catalogue = releases::agent_versions(&fixture.state, None, None).await?;
    assert_eq!(catalogue[0].cached_targets, ["amd64", "arm64"]);
    no_staging(&fixture.release_root())?;
    Ok(())
}

#[tokio::test]
async fn failed_downloads_or_unsigned_identities_do_not_change_existing_inventory() -> Result<()> {
    let fixture = Fixture::new()?;
    let bundle = targets::multi_arch_bundle()?;
    targets::import_target(&fixture.state, &bundle, "arm64", &Mutex::new(Vec::new())).await?;
    let before = snapshot(&fixture.release_root())?;
    for tampered in [false, true] {
        let result =
            releases::cache_agent_payload(&fixture.state, "0.3.0", "amd64", |_, _, _| async move {
                if tampered {
                    Ok(b"tampered fixture".to_vec())
                } else {
                    bail!("interrupted download")
                }
            })
            .await;
        assert!(matches!(result, Err(ApiError::Conflict(_))));
        assert_eq!(snapshot(&fixture.release_root())?, before);
    }
    assert!(matches!(
        releases::cache_agent_payload(&fixture.state, "99.0.0", "amd64", |_, _, _| async {
            bail!("an unsigned identity must never fetch an asset")
        })
        .await,
        Err(ApiError::NotFound)
    ));
    no_staging(&fixture.release_root())?;
    Ok(())
}

#[tokio::test]
async fn corrupt_cached_agent_remains_selectable_and_can_be_repaired_on_demand() -> Result<()> {
    let fixture = Fixture::new()?;
    let bundle = targets::multi_arch_bundle()?;
    targets::import_target(&fixture.state, &bundle, "arm64", &Mutex::new(Vec::new())).await?;
    let path = fixture
        .release_root()
        .join(&bundle.tag)
        .join("agent/0.3.0/arm64");
    std::fs::write(&path, b"damaged")?;
    let catalogue =
        releases::agent_versions(&fixture.state, Some("linux-musl-arm64"), None).await?;
    assert_eq!(catalogue[0].targets, ["arm64"]);
    assert!(catalogue[0].cached_targets.is_empty());
    releases::cache_agent_payload(&fixture.state, "0.3.0", "arm64", |_, name, _| {
        let bytes = bundle
            .assets
            .get(&name)
            .cloned()
            .context("fixture asset missing");
        async move { bytes }
    })
    .await?;
    assert_eq!(std::fs::read(&path)?, b"arm agent");
    assert_eq!(releases::entries(&fixture.state).await?.len(), 2);
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn on_demand_cache_rejects_symlinks_without_touching_external_files() -> Result<()> {
    let fixture = Fixture::new()?;
    let bundle = targets::multi_arch_bundle()?;
    targets::import_target(&fixture.state, &bundle, "arm64", &Mutex::new(Vec::new())).await?;
    let outside = fixture.root.join("outside");
    std::fs::write(&outside, b"preserve")?;
    let path = fixture
        .release_root()
        .join(&bundle.tag)
        .join("agent/0.3.0/amd64");
    std::os::unix::fs::symlink(&outside, &path)?;
    assert!(matches!(
        releases::cache_agent_payload(&fixture.state, "0.3.0", "amd64", |_, _, _| async {
            bail!("unsafe paths must be rejected before download")
        })
        .await,
        Err(ApiError::Conflict(_))
    ));
    assert_eq!(std::fs::read(&outside)?, b"preserve");
    assert!(std::fs::symlink_metadata(path)?.is_symlink());
    Ok(())
}

#[tokio::test]
async fn concurrent_cache_requests_serialize_and_download_one_agent_once() -> Result<()> {
    let fixture = Fixture::new()?;
    let bundle = targets::multi_arch_bundle()?;
    targets::import_target(&fixture.state, &bundle, "arm64", &Mutex::new(Vec::new())).await?;
    let count = Mutex::new(0);
    let requests = (0..3).map(|_| {
        releases::cache_agent_payload(&fixture.state, "0.3.0", "amd64", |_, name, _| {
            *count.lock().unwrap() += 1;
            let bytes = bundle
                .assets
                .get(&name)
                .cloned()
                .context("fixture asset missing");
            async move { bytes }
        })
    });
    for result in futures_util::future::join_all(requests).await {
        result?;
    }
    assert_eq!(*count.lock().unwrap(), 1);
    assert_eq!(releases::entries(&fixture.state).await?.len(), 3);
    no_staging(&fixture.release_root())?;
    Ok(())
}

#[tokio::test]
async fn native_agents_can_be_cached_from_the_same_signed_catalogue() -> Result<()> {
    let fixture = Fixture::new()?;
    let native_targets = ["windows-arm64", "macos-arm64", "freebsd-arm64"];
    let bundle = platform_bundle(
        "0.3.0",
        (1, 1),
        &[
            "linux-musl-arm64",
            "windows-arm64",
            "macos-arm64",
            "freebsd-arm64",
        ],
    )?;
    targets::import_target(
        &fixture.state,
        &bundle,
        "linux-musl-arm64",
        &Mutex::new(Vec::new()),
    )
    .await?;
    for target in native_targets {
        let versions = releases::agent_versions(&fixture.state, Some(target), None).await?;
        assert_eq!(versions[0].targets, [target]);
        assert!(versions[0].cached_targets.is_empty());
        releases::cache_agent_payload(&fixture.state, "0.3.0", target, |tag, name, _| {
            assert_eq!(tag, bundle.tag);
            assert!(name.ends_with(target));
            let bytes = bundle
                .assets
                .get(&name)
                .cloned()
                .context("native fixture missing");
            async move { bytes }
        })
        .await?;
        let versions = releases::agent_versions(&fixture.state, Some(target), None).await?;
        assert_eq!(versions[0].cached_targets, [target]);
        assert!(
            !releases::artifact(&fixture.state, "agent", "0.3.0", target)
                .await?
                .0
                .is_empty()
        );
    }
    assert_eq!(releases::entries(&fixture.state).await?.len(), 4);
    Ok(())
}
