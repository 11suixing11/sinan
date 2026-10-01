use super::*;
use crate::{
    IpVersion, run,
    tests::fixture::{Directory, target},
};
use std::{
    fs as std_fs,
    os::unix::fs::{OpenOptionsExt, PermissionsExt, symlink},
    sync::Arc,
};
use tokio::sync::Notify;

fn write_private(path: &Path, bytes: &[u8]) {
    use std::io::Write;
    let mut file = std_fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
}

fn output(directory: &Directory) -> (PathBuf, PathBuf) {
    let result = directory.path.join("result.json");
    let pending = directory.path.join(".result.json.pending");
    write_private(&result, b"previous complete result");
    write_private(
        &directory.path.join("sections/tcp_scope.json"),
        b"previous complete section",
    );
    (result, pending)
}

fn assert_old_output(directory: &Directory, result: &Path) {
    assert_eq!(std_fs::read(result).unwrap(), b"previous complete result");
    assert_eq!(
        std_fs::read(directory.path.join("sections/tcp_scope.json")).unwrap(),
        b"previous complete section"
    );
}

#[tokio::test]
async fn foreign_workspace_owner_is_rejected_before_report_data_is_written() {
    let directory = Directory::new();
    let (options, mut journal) = directory
        .prepare(vec![target(1, "127.0.0.1", 1)], IpVersion::V4)
        .await;
    // Model the captured owner of a foreign-UID workspace without chown/root.
    journal.owner_uid ^= 1;
    let error = run(&options, &mut journal).await.unwrap_err();
    assert!(error.to_string().contains("effective user"));
    assert!(!directory.path.join("result.json").exists());
    assert!(
        !directory
            .path
            .join("sections/.tcp_scope.json.pending")
            .exists()
    );
    assert_eq!(
        std_fs::read_dir(directory.path.join("sections"))
            .unwrap()
            .count(),
        0
    );
}

#[tokio::test]
async fn successful_publication_removes_pending_and_keeps_prior_sections() {
    let directory = Directory::new();
    let (_, journal) = directory
        .prepare(vec![target(1, "127.0.0.1", 1)], IpVersion::V4)
        .await;
    let (result, pending) = output(&directory);
    journal
        .atomic(&result, b"new complete result", Instant::now() + IO_LIMIT)
        .await
        .unwrap();
    assert_eq!(std_fs::read(result).unwrap(), b"new complete result");
    assert!(!pending.exists());
    assert_eq!(
        std_fs::read(directory.path.join("sections/tcp_scope.json")).unwrap(),
        b"previous complete section"
    );
    assert_eq!(
        std_fs::metadata(directory.path.join("result.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}

#[tokio::test]
async fn write_sync_and_commit_errors_remove_owned_pending_without_losing_reports() {
    for point in [
        PublicationFault::Write,
        PublicationFault::Sync,
        PublicationFault::Rename,
    ] {
        let directory = Directory::new();
        let (_, journal) = directory
            .prepare(vec![target(1, "127.0.0.1", 1)], IpVersion::V4)
            .await;
        let (result, pending) = output(&directory);
        *journal.publication_fault.lock().unwrap() = Some(point);
        let error = journal
            .atomic(&result, b"replacement result", Instant::now() + IO_LIMIT)
            .await
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("injected publication I/O failure")
        );
        assert!(!pending.exists());
        assert_old_output(&directory, &result);
        // An error leaves no stale pending name that could block a later attempt.
        journal
            .atomic(&result, b"retry result", Instant::now() + IO_LIMIT)
            .await
            .unwrap();
        assert_eq!(std_fs::read(result).unwrap(), b"retry result");
        assert!(!pending.exists());
    }
}

#[tokio::test]
async fn real_rename_failure_removes_pending_and_preserves_unrelated_files() {
    let directory = Directory::new();
    let (_, journal) = directory
        .prepare(vec![target(1, "127.0.0.1", 1)], IpVersion::V4)
        .await;
    let (result, _) = output(&directory);
    let occupied = directory.path.join("occupied.json");
    std_fs::create_dir(&occupied).unwrap();
    let error = journal
        .atomic(&occupied, b"replacement", Instant::now() + IO_LIMIT)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("publish report"));
    assert!(occupied.is_dir());
    assert!(!directory.path.join(".occupied.json.pending").exists());
    assert_old_output(&directory, &result);
}

#[tokio::test]
async fn publication_timeout_cleans_the_owned_file_before_returning() {
    let directory = Directory::new();
    let (_, mut journal) = directory
        .prepare(vec![target(1, "127.0.0.1", 1)], IpVersion::V4)
        .await;
    let (result, pending) = output(&directory);
    let entered = Arc::new(Notify::new());
    journal.gate_publication_after(0, entered.clone(), Arc::new(Notify::new()));
    let operation = journal.atomic(
        &result,
        b"uncommitted replacement",
        Instant::now() + Duration::from_millis(300),
    );
    tokio::pin!(operation);
    tokio::select! {
        _ = entered.notified() => {},
        result = &mut operation => panic!("publication never reached its I/O gate: {result:?}"),
    }
    assert!(pending.exists());
    let error = operation.await.unwrap_err();
    assert!(error.to_string().contains("timed out"));
    assert!(!pending.exists());
    assert_old_output(&directory, &result);
}

#[tokio::test]
async fn cancelling_a_written_publication_cleans_pending_and_does_not_commit_later() {
    let directory = Directory::new();
    let (_, mut journal) = directory
        .prepare(vec![target(1, "127.0.0.1", 1)], IpVersion::V4)
        .await;
    let (result, pending) = output(&directory);
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    journal.gate_publication_after(0, entered.clone(), release.clone());
    let target = result.clone();
    let task = tokio::spawn(async move {
        journal
            .atomic(&target, b"cancelled replacement", Instant::now() + IO_LIMIT)
            .await
    });
    tokio::time::timeout(IO_LIMIT, entered.notified())
        .await
        .unwrap();
    assert!(pending.exists());
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert!(!pending.exists());
    assert_old_output(&directory, &result);
    // Releasing a gate after the cancelled future has been joined cannot queue
    // a delayed rename or affect an already published section/result.
    release.notify_waiters();
    tokio::task::yield_now().await;
    assert_old_output(&directory, &result);
    assert!(!pending.exists());
}

#[tokio::test]
async fn preexisting_pending_file_is_not_deleted_or_modified() {
    let directory = Directory::new();
    let (_, journal) = directory
        .prepare(vec![target(1, "127.0.0.1", 1)], IpVersion::V4)
        .await;
    let (result, pending) = output(&directory);
    write_private(&pending, b"not owned by this publication");
    assert!(
        journal
            .atomic(&result, b"replacement", Instant::now() + IO_LIMIT)
            .await
            .is_err()
    );
    assert_eq!(
        std_fs::read(pending).unwrap(),
        b"not owned by this publication"
    );
    assert_old_output(&directory, &result);
}

#[test]
fn pending_cleanup_does_not_follow_or_delete_a_replacement_object() {
    for symbolic in [false, true] {
        let directory = Directory::new();
        let pending = directory.path.join(".result.json.pending");
        let unrelated = directory.path.join("unrelated.json");
        write_private(&unrelated, b"unrelated private data");
        let mut owned = PendingFile::create(&pending).unwrap();
        std_fs::remove_file(&pending).unwrap();
        if symbolic {
            symlink(&unrelated, &pending).unwrap();
        } else {
            write_private(&pending, b"foreign replacement");
        }
        assert!(owned.publish(&directory.path.join("result.json")).is_err());
        owned.cleanup().unwrap();
        drop(owned);
        assert!(std_fs::symlink_metadata(&pending).is_ok());
        assert_eq!(std_fs::read(&unrelated).unwrap(), b"unrelated private data");
        if !symbolic {
            assert_eq!(std_fs::read(pending).unwrap(), b"foreign replacement");
        }
        assert!(!directory.path.join("result.json").exists());
    }
}

#[test]
fn pending_cleanup_unlinks_only_its_name_and_preserves_an_unrelated_hard_link() {
    let directory = Directory::new();
    let pending = directory.path.join(".result.json.pending");
    let alias = directory.path.join("unrelated-link");
    let mut owned = PendingFile::create(&pending).unwrap();
    std_fs::hard_link(&pending, &alias).unwrap();
    assert!(owned.publish(&directory.path.join("result.json")).is_err());
    owned.cleanup().unwrap();
    assert!(!pending.exists());
    assert!(alias.exists());
    assert!(!directory.path.join("result.json").exists());
}
