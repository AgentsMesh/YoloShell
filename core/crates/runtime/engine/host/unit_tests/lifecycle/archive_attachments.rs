use super::*;
use crate::paths::attachments::directory;
use std::fs;

const STAGED_BYTES: &[u8] = b"staged";
const HISTORY_BYTES: &[u8] = b"retained output";

fn stage(host: &Host, id: Uuid) -> PathBuf {
    let root = directory(&host.paths.session_dir(id));
    let epoch = root.join(Uuid::new_v4().to_string());
    for state in ["pending", "files"] {
        let upload = epoch.join(Uuid::new_v4().to_string()).join(state);
        host.namespace.ensure_directory(&upload).unwrap();
        write_private_bytes(&upload.join("notes.txt"), STAGED_BYTES);
    }
    write_private_bytes(&host.paths.journal(id), HISTORY_BYTES);
    root
}

#[tokio::test]
async fn startup_reclaims_partial_and_published_uploads_from_durable_archives() {
    let home = tempdir().unwrap();
    let first = Host::open(HostConfig::new(home.path())).unwrap();
    let active = fixture_manifest(home.path());
    seed_recovery_manifest(&first, &active).await;
    let active_root = stage(&first, active.terminal_id);
    let mut archives = Vec::new();
    for indexed in [Some(false), Some(true), None] {
        let mut manifest = fixture_manifest(home.path());
        let id = manifest.terminal_id;
        first.paths.ensure_session(id).unwrap();
        if let Some(archived) = indexed {
            manifest.archived = archived;
            first.catalog_insert(&manifest).await.unwrap();
        }
        manifest.archived = true;
        first
            .write_namespace_json(&first.paths.manifest(id), &manifest)
            .unwrap();
        archives.push((id, stage(&first, id)));
    }
    drop(first);

    let host = Host::open(HostConfig::new(home.path())).unwrap();
    host.reconcile().await;
    for (id, root) in archives {
        assert!(!root.exists());
        assert_eq!(fs::read(host.paths.journal(id)).unwrap(), HISTORY_BYTES);
    }
    assert!(active_root.is_dir());
    assert!(lock_recover(&host.pending_attachment_cleanup).is_empty());
}

#[tokio::test]
async fn foreground_archive_retries_failed_cleanup_and_preserves_symlink_target() {
    let home = tempdir().unwrap();
    let host = Host::open(HostConfig::new(home.path())).unwrap();
    let manifest = fixture_manifest(home.path());
    let id = manifest.terminal_id;
    seed_recovery_manifest(&host, &manifest).await;
    let root = stage(&host, id);
    let retained = home.path().join("retained");
    fs::rename(&root, &retained).unwrap();
    std::os::unix::fs::symlink(&retained, &root).unwrap();
    assert!(host.archive_bound_session(id).await.is_err());
    assert!(host.load_durable_manifest(id).unwrap().archived);
    assert_eq!(host.retry_archived_attachment_cleanup().await, 1);
    assert!(root.is_symlink());
    fs::remove_file(&root).unwrap();
    fs::rename(retained, &root).unwrap();
    host.reconcile().await;
    assert!(root.is_dir());
    assert!(
        host.archive_bound_session(id)
            .await
            .unwrap()
            .manifest
            .archived
    );
    assert!(!root.exists());
    assert_eq!(fs::read(host.paths.journal(id)).unwrap(), HISTORY_BYTES);
}

#[tokio::test]
async fn ordinary_archive_removes_staging_only_after_its_selection_is_accepted() {
    let home = tempdir().unwrap();
    let host = Host::open(HostConfig::new(home.path())).unwrap();
    let manifest = fixture_manifest(home.path());
    let id = manifest.terminal_id;
    seed_recovery_manifest(&host, &manifest).await;
    let root = stage(&host, id);
    let mut stale = TerminalLifecycleSelection::unconditional(id);
    let history = host.paths.journal(id);
    std::os::unix::fs::symlink(&history, root.join("history-link")).unwrap();
    stale.expected_runtime_epoch = Some(Uuid::new_v4());
    assert!(host.archive_product_session(stale).await.is_err());
    assert!(root.is_dir());
    assert!(!host.load_durable_manifest(id).unwrap().archived);
    let archived = host
        .archive_product_session(TerminalLifecycleSelection::unconditional(id))
        .await
        .unwrap();
    assert!(archived.manifest.archived);
    assert!(!root.exists());
    assert_eq!(fs::read(host.paths.journal(id)).unwrap(), HISTORY_BYTES);
}

#[tokio::test]
async fn parent_sync_failure_after_unlink_retries_even_when_the_tree_is_absent() {
    let home = tempdir().unwrap();
    let first = Host::open(HostConfig::new(home.path())).unwrap();
    let mut manifest = fixture_manifest(home.path());
    manifest.archived = true;
    let id = manifest.terminal_id;
    seed_recovery_manifest(&first, &manifest).await;
    let root = directory(&first.paths.session_dir(id));
    first.namespace.ensure_directory(&root).unwrap();
    drop(
        first
            .paths
            .try_acquire_session_lifecycle_lock(id)
            .unwrap()
            .unwrap(),
    );
    drop(first);
    let host = Host::open(HostConfig::new(home.path())).unwrap();
    crate::paths::test_support::fail_next_directory_sync();
    assert_eq!(host.retry_archived_attachment_cleanup().await, 1);
    assert!(!root.exists());
    assert!(host.prepare_idle_shutdown().await.is_err());
    crate::paths::test_support::fail_next_directory_sync();
    assert_eq!(host.retry_archived_attachment_cleanup().await, 1);
    assert_eq!(host.retry_archived_attachment_cleanup().await, 0);
    assert!(lock_recover(&host.pending_attachment_cleanup).is_empty());
    assert!(host.prepare_idle_shutdown().await.is_ok());
}

#[path = "archive_attachment_fences.rs"]
mod fences;
