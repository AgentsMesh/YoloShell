use crate::host::tests::*;
use super::*;
pub(super) const CORRUPT_CATALOG_BYTES: &[u8] = b"not a sqlite database";
const TRUNCATED_CATALOG_BYTES: &[u8] = b"SQLite format 3\0";
const WRONG_SCHEMA_SQL: &str = "CREATE TABLE metadata (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL); INSERT INTO metadata(key, value) VALUES ('schema_version', '999');";
pub(super) const QUARANTINE_PREFIX: &str = "catalog.sqlite.quarantine-";
const CONCURRENT_STARTS: usize = 6;

#[test]
fn corrupt_catalog_is_quarantined_and_rebuilt_from_durable_manifests() {
    let directory = tempdir().unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    let manifest_path = Paths::new(directory.path()).manifest(manifest.terminal_id);
    let before = std::fs::read(&manifest_path).unwrap();
    std::fs::write(
        directory.path().join("catalog.sqlite"),
        CORRUPT_CATALOG_BYTES,
    )
    .unwrap();

    let host = Host::open(HostConfig::new(directory.path())).unwrap();

    assert_rebuilt_manifest(&host, &manifest);
    assert_eq!(std::fs::read(manifest_path).unwrap(), before);
    assert_quarantined_catalog(directory.path());
}

#[test]
fn truncated_catalog_is_rebuilt_without_removing_session_state() {
    let directory = tempdir().unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    std::fs::write(
        directory.path().join("catalog.sqlite"),
        TRUNCATED_CATALOG_BYTES,
    )
    .unwrap();

    let host = Host::open(HostConfig::new(directory.path())).unwrap();

    assert_rebuilt_manifest(&host, &manifest);
    assert!(
        Paths::new(directory.path())
            .session_dir(manifest.terminal_id)
            .is_dir()
    );
}

#[test]
fn rebuilt_catalog_advances_past_durable_worker_controller_generation() {
    const DURABLE_WORKER_GENERATION: u64 = 8;

    let directory = tempdir().unwrap();
    let original = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    let mut record = host_runtime_record(
        &original,
        manifest.terminal_id,
        directory.path().join("worker.sock"),
    );
    record.controller_generation = DURABLE_WORKER_GENERATION;
    atomic_write_json(&original.paths.runtime_record(manifest.terminal_id), &record).unwrap();
    drop(original);
    std::fs::write(
        directory.path().join("catalog.sqlite"),
        CORRUPT_CATALOG_BYTES,
    )
    .unwrap();

    let host = Host::open(HostConfig::new(directory.path())).unwrap();

    assert_rebuilt_manifest(&host, &manifest);
    assert_eq!(host.controller_generation(), DURABLE_WORKER_GENERATION + 1);
}

#[test]
fn unsupported_catalog_schema_is_rebuilt_from_durable_manifests() {
    let directory = tempdir().unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    let path = directory.path().join("catalog.sqlite");
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute_batch(WRONG_SCHEMA_SQL)
        .unwrap();

    let host = Host::open(HostConfig::new(directory.path())).unwrap();

    assert_rebuilt_manifest(&host, &manifest);
    assert_quarantined_catalog(directory.path());
}

#[cfg(unix)]
#[test]
fn catalog_symlink_is_quarantined_without_following_its_target() {
    use std::os::unix::fs::symlink;

    let directory = tempdir().unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    let outside = directory.path().join("outside.sqlite");
    let outside_bytes = b"do not modify this file";
    std::fs::write(&outside, outside_bytes).unwrap();
    let outside_wal = directory.path().join("outside-wal.sqlite");
    std::fs::write(&outside_wal, outside_bytes).unwrap();
    symlink(&outside, directory.path().join("catalog.sqlite")).unwrap();
    symlink(&outside_wal, directory.path().join("catalog.sqlite-wal")).unwrap();

    let host = Host::open(HostConfig::new(directory.path())).unwrap();

    assert_rebuilt_manifest(&host, &manifest);
    assert_eq!(std::fs::read(outside).unwrap(), outside_bytes);
    assert_eq!(std::fs::read(outside_wal).unwrap(), outside_bytes);
    assert_quarantined_catalog(directory.path());
}

#[cfg(unix)]
#[test]
fn catalog_hard_link_is_rebuilt_without_modifying_its_other_name() {
    let directory = tempdir().unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    let outside = directory.path().join("outside.sqlite");
    let outside_bytes = b"do not modify this file";
    std::fs::write(&outside, outside_bytes).unwrap();
    std::fs::hard_link(&outside, directory.path().join("catalog.sqlite")).unwrap();

    let host = Host::open(HostConfig::new(directory.path())).unwrap();

    assert_rebuilt_manifest(&host, &manifest);
    assert_eq!(std::fs::read(outside).unwrap(), outside_bytes);
    assert_quarantined_catalog(directory.path());
}

#[test]
fn concurrent_starts_rebuild_a_corrupt_catalog_once_and_preserve_manifests() {
    use std::sync::Barrier;

    let directory = tempdir().unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    std::fs::write(
        directory.path().join("catalog.sqlite"),
        CORRUPT_CATALOG_BYTES,
    )
    .unwrap();
    let start = Arc::new(Barrier::new(CONCURRENT_STARTS));
    let finish = Arc::new(Barrier::new(CONCURRENT_STARTS));
    let handles: Vec<_> = (0..CONCURRENT_STARTS)
        .map(|_| {
            let home = directory.path().to_path_buf();
            let start = Arc::clone(&start);
            let finish = Arc::clone(&finish);
            std::thread::spawn(move || {
                start.wait();
                let host = Host::open(HostConfig::new(home));
                let acquired = host.is_ok();
                finish.wait();
                acquired
            })
        })
        .collect();
    let acquired = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .filter(|acquired| *acquired)
        .count();

    assert_eq!(acquired, 1);
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    assert_rebuilt_manifest(&host, &manifest);
}

pub(super) fn persist_rebuild_manifest(home: &Path) -> TerminalManifest {
    let paths = Paths::new(home);
    let manifest = fixture_manifest(home);
    paths.ensure_session(manifest.terminal_id).unwrap();
    atomic_write_json(&paths.manifest(manifest.terminal_id), &manifest).unwrap();
    manifest
}

pub(super) fn assert_rebuilt_manifest(host: &Host, manifest: &TerminalManifest) {
    let restored = lock_recover(&host.catalog)
        .get(manifest.terminal_id)
        .unwrap()
        .unwrap();
    assert_eq!(restored.terminal_id, manifest.terminal_id);
    assert_eq!(restored.recovery_state, manifest.recovery_state);
}

pub(super) fn assert_quarantined_catalog(home: &Path) {
    let quarantined = std::fs::read_dir(home)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .any(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(QUARANTINE_PREFIX)
        });
    assert!(quarantined);
}
