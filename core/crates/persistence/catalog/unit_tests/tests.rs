use super::*;
use crate::model::{DEFAULT_COLS, DEFAULT_ROWS};
use tempfile::tempdir;

fn manifest(id: Uuid, archived: bool) -> TerminalManifest {
    TerminalManifest {
        schema_version: SESSION_MANIFEST_SCHEMA_VERSION,
        terminal_id: id,
        name: "test".into(),
        command: "/bin/sh".into(),
        args: vec!["-l".into()],
        cwd: std::env::current_dir().unwrap(),
        cols: DEFAULT_COLS,
        rows: DEFAULT_ROWS,
        created_at_ms: now_ms(),
        updated_at_ms: now_ms(),
        archived,
        recovery_state: RecoveryState::Created,
    }
}

#[test]
fn insert_get_list_archive_and_delete() {
    let dir = tempdir().unwrap();
    let mut catalog = Catalog::open(&dir.path().join("catalog.sqlite")).unwrap();
    let first = manifest(Uuid::new_v4(), false);
    let second = manifest(Uuid::new_v4(), true);
    catalog.insert(&first).unwrap();
    catalog.insert(&second).unwrap();
    assert_eq!(
        catalog.get(first.terminal_id).unwrap().unwrap().command,
        "/bin/sh"
    );
    assert_eq!(catalog.list(false).unwrap().len(), 1);
    assert_eq!(catalog.list(true).unwrap().len(), 2);
    assert!(
        catalog
            .archive(first.terminal_id, true)
            .unwrap()
            .unwrap()
            .archived
    );
    assert!(catalog.list(false).unwrap().is_empty());
    assert!(catalog.delete(second.terminal_id).unwrap());
    assert!(!catalog.delete(second.terminal_id).unwrap());
    catalog.integrity_check().unwrap();
}

#[test]
fn generation_is_monotonic_across_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("catalog.sqlite");
    {
        let mut catalog = Catalog::open(&path).unwrap();
        assert_eq!(catalog.bump_controller_generation().unwrap(), 1);
        assert_eq!(catalog.bump_controller_generation().unwrap(), 2);
    }
    let mut reopened = Catalog::open(&path).unwrap();
    assert_eq!(reopened.controller_generation().unwrap(), 2);
    assert_eq!(reopened.bump_controller_generation().unwrap(), 3);
}

#[test]
fn duplicate_insert_is_rejected_but_upsert_updates() {
    let dir = tempdir().unwrap();
    let mut catalog = Catalog::open(&dir.path().join("catalog.sqlite")).unwrap();
    let mut item = manifest(Uuid::new_v4(), false);
    catalog.insert(&item).unwrap();
    assert!(catalog.insert(&item).is_err());
    item.name = "renamed".into();
    catalog.upsert(&item).unwrap();
    assert_eq!(
        catalog.get(item.terminal_id).unwrap().unwrap().name,
        "renamed"
    );
}

#[test]
fn tolerant_listing_preserves_and_diagnoses_a_corrupt_row() {
    const CORRUPT_ROW_SQL: &str = "UPDATE sessions SET manifest_json = ?1 WHERE session_id = ?2";
    const ROW_COUNT_SQL: &str = "SELECT COUNT(*) FROM sessions";
    let dir = tempdir().unwrap();
    let mut catalog = Catalog::open(&dir.path().join("catalog.sqlite")).unwrap();
    let valid = manifest(Uuid::new_v4(), false);
    let corrupt = manifest(Uuid::new_v4(), false);
    catalog.insert(&valid).unwrap();
    catalog.insert(&corrupt).unwrap();
    catalog
        .connection
        .execute(
            CORRUPT_ROW_SQL,
            rusqlite::params![b"not-json", corrupt.terminal_id.to_string()],
        )
        .unwrap();
    let listing = catalog.list_tolerant(false).unwrap();
    assert_eq!(listing.manifests.len(), 1);
    assert_eq!(listing.manifests[0].terminal_id, valid.terminal_id);
    assert_eq!(listing.diagnostics.len(), 1);
    assert_eq!(listing.diagnostics[0].terminal_id, Some(corrupt.terminal_id));
    assert!(catalog.list(false).is_err());
    let retained: usize = catalog
        .connection
        .query_row(ROW_COUNT_SQL, [], |row| row.get(0))
        .unwrap();
    assert_eq!(retained, 2);
}

#[test]
fn unsupported_session_schema_is_rejected_on_write_and_tolerant_read() {
    const REPLACE_ROW_SQL: &str = "UPDATE sessions SET manifest_json = ?1 WHERE session_id = ?2";
    let dir = tempdir().unwrap();
    let mut catalog = Catalog::open(&dir.path().join("catalog.sqlite")).unwrap();
    let valid = manifest(Uuid::new_v4(), false);
    catalog.insert(&valid).unwrap();
    let mut unsupported = valid.clone();
    unsupported.schema_version = SESSION_MANIFEST_SCHEMA_VERSION + 1;
    assert!(catalog.upsert(&unsupported).is_err());
    let encoded = serde_json::to_vec(&unsupported).unwrap();
    catalog
        .connection
        .execute(
            REPLACE_ROW_SQL,
            rusqlite::params![encoded, valid.terminal_id.to_string()],
        )
        .unwrap();
    let listing = catalog.list_tolerant(false).unwrap();
    assert!(listing.manifests.is_empty());
    assert_eq!(listing.diagnostics.len(), 1);
    assert_eq!(listing.diagnostics[0].terminal_id, Some(valid.terminal_id));
    assert!(catalog.get(valid.terminal_id).is_err());
}

#[test]
fn noncanonical_text_schema_versions_are_rebuildable() {
    const NONCANONICAL_VERSIONS: &[&str] = &["1junk", "", " 1", "+1", "01", "1.0", "1e0"];

    for version in NONCANONICAL_VERSIONS {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("catalog.sqlite");
        drop(Catalog::open(&path).unwrap());
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute(
                "UPDATE metadata SET value = ?1 WHERE key = 'schema_version'",
                [version],
            )
            .unwrap();

        let error = Catalog::open(&path).err().unwrap();

        assert!(
            rebuild_reason::is_rebuildable(&error),
            "version {version:?}"
        );
    }
}

#[test]
fn blob_schema_version_is_rebuildable() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("catalog.sqlite");
    drop(Catalog::open(&path).unwrap());
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE metadata SET value = ?1 WHERE key = 'schema_version'",
            [b"1".as_slice()],
        )
        .unwrap();

    let error = Catalog::open(&path).err().unwrap();

    assert!(rebuild_reason::is_rebuildable(&error));
}

#[test]
fn recovery_and_missing_updates_are_explicit() {
    let dir = tempdir().unwrap();
    let mut catalog = Catalog::open(&dir.path().join("catalog.sqlite")).unwrap();
    let item = manifest(Uuid::new_v4(), false);
    catalog.insert(&item).unwrap();
    let updated = catalog
        .update_recovery(item.terminal_id, RecoveryState::Uncertain)
        .unwrap()
        .unwrap();
    assert_eq!(updated.recovery_state, RecoveryState::Uncertain);
    let missing = Uuid::new_v4();
    assert!(
        catalog
            .update_recovery(missing, RecoveryState::Resumed)
            .unwrap()
            .is_none()
    );
    assert!(catalog.archive(missing, true).unwrap().is_none());
    assert!(catalog.get(missing).unwrap().is_none());
}
