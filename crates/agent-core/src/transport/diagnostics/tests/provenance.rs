use super::*;

const AUXILIARY_NAMES: [&str; 4] = ["build-info.json", "LICENSE", "source.tar.gz", "Cargo.lock"];

struct ProvenanceAdapter {
    declared: bool,
    tamper: bool,
}

impl DiagnosticAdapter for ProvenanceAdapter {
    fn auxiliary_files(&self) -> Vec<String> {
        if self.declared {
            AUXILIARY_NAMES.into_iter().map(str::to_owned).collect()
        } else {
            Vec::new()
        }
    }
    fn describe(&self) -> DiagnosticDescriptor {
        TestAdapter.describe()
    }
    fn prepare<'a>(
        &'a self,
        spec: &'a DiagnosticSpec,
        privileged: &'a dyn Privileged,
    ) -> BoxFuture<'a, ServiceJob> {
        Box::pin(async move {
            let service = TestAdapter.prepare(spec, privileged).await?;
            if self.tamper {
                std::fs::write(
                    spec.binary_path.parent().unwrap().join("source.tar.gz"),
                    b"changed",
                )?;
            }
            Ok(service)
        })
    }
    fn collect<'a>(&'a self, spec: &'a DiagnosticSpec) -> BoxFuture<'a, Option<DiagnosticOutput>> {
        TestAdapter.collect(spec)
    }
}

fn cached_provenance(directory: &Directory, id: Uuid) -> Result<DiagnosticJob> {
    let mut job = deadline::cached_job(directory, id)?;
    let artifact = directory
        .config()
        .install_root
        .join("diagnostic-fixture/v1");
    let binary = std::fs::read(artifact.join("runner"))?;
    let mut entry = release_support::entry(
        "diagnostic-fixture",
        "v1",
        "runner",
        "tar.gz",
        b"cached archive",
        &binary,
    );
    for name in AUXILIARY_NAMES {
        let content = format!("TEST_ONLY diagnostic provenance {name}");
        std::fs::write(artifact.join(name), content.as_bytes())?;
        entry.auxiliary_files.insert(
            name.into(),
            sinan_protocol::release::ReleaseFile {
                sha256: release_support::hash(content.as_bytes()),
                size: content.len() as u64,
            },
        );
    }
    let proof = release_support::signed_release(vec![(entry, b"cached archive".to_vec())]);
    release_support::install_proof(&artifact, &proof);
    job.artifact.proof = Some(proof);
    Ok(job)
}

#[tokio::test]
async fn diagnostic_provenance_files_are_required_and_rechecked_before_service_start() -> Result<()>
{
    for (declared, tamper, missing, expected_starts) in [
        (true, false, false, 1),
        (false, false, false, 0),
        (true, false, true, 0),
        (true, true, false, 0),
    ] {
        let directory = Directory::new();
        let config = directory.config();
        let services = Arc::new(Services::new(JobStatus::Running));
        let job = cached_provenance(&directory, Uuid::new_v4())?;
        if missing {
            std::fs::remove_file(config.install_root.join("diagnostic-fixture/v1/LICENSE"))?;
        }
        let worker = DiagnosticWorker::new(
            config.clone(),
            Arc::new(Mutex::new(State::open(&config.state_db)?)),
            vec![Arc::new(ProvenanceAdapter { declared, tamper })],
            Arc::new(FakeResourceOps::new(Arc::new(SystemOps))),
            services.clone(),
        )?
        .with_trusted_keys(release_support::trusted_keys());
        worker.save(&Checkpoint::Preparing(job))?;
        let client = PanelClient::new("http://127.0.0.1:1", "test-session")?
            .with_trusted_keys(release_support::trusted_keys());
        worker.tick(Some(&client)).await?;
        assert_eq!(
            services.starts.load(Ordering::Relaxed),
            expected_starts,
            "declared={declared}, tamper={tamper}, missing={missing}"
        );
        if expected_starts == 0 {
            assert!(worker.active()?.is_none());
            let pending: Vec<DiagnosticUpdate> = worker.read(OUTBOX)?.unwrap();
            assert_eq!(pending[0].status, DiagnosticStatus::Failed);
        } else {
            assert!(matches!(worker.active()?, Some(Checkpoint::Started { .. })));
        }
    }
    Ok(())
}
