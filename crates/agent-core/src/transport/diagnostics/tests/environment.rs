use super::*;
use crate::transport::diagnostics::environment::ExecutionEnvironment;
use sinan_adapter_sdk::{DiagnosticMemory, DiagnosticResources};
use sinan_protocol::DiagnosticSectionUpdate;

#[tokio::test]
async fn environment_checkpoint_survives_restart_and_replays_the_same_section() -> Result<()> {
    let directory = Directory::new();
    let services = Arc::new(Services::new(JobStatus::Running));
    let first = worker(&directory, services.clone())?;
    let mut saved = checkpoint(&first.config, Uuid::new_v4());
    let Checkpoint::Started {
        spec,
        service,
        environment,
        ..
    } = &mut saved
    else {
        unreachable!()
    };
    spec.options
        .insert("environment_section".into(), "true".into());
    *environment = Some(ExecutionEnvironment::capture(
        service,
        &DiagnosticResources {
            memory: DiagnosticMemory {
                host_available_bytes: 1024 * 1024 * 1024,
                cgroup_available_bytes: Some(900 * 1024 * 1024),
            },
            disk_available_bytes: 8 * 1024 * 1024 * 1024,
            load_one: 0.75,
            cpu_count: 2,
        },
        1700000000,
    ));
    first.save(&saved)?;
    first.capture_environment(&saved)?;
    let original: Vec<DiagnosticSectionUpdate> = first.read(sections::SECTIONS_OUTBOX)?.unwrap();
    assert_eq!(original.len(), 1);
    assert!(original[0].text.contains("MemoryMax：536870912"));
    assert!(original[0].text.contains("一分钟负载：0.750"));
    drop(first);
    let recovered = worker(&directory, services.clone())?;
    recovered.capture_environment(&recovered.active()?.unwrap())?;
    let replay: Vec<DiagnosticSectionUpdate> = recovered.read(sections::SECTIONS_OUTBOX)?.unwrap();
    assert_eq!(original, replay);
    assert_eq!(services.starts.load(std::sync::atomic::Ordering::SeqCst), 0);
    Ok(())
}
