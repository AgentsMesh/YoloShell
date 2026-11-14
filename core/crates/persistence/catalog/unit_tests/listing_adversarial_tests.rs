use super::*;
use crate::model::{DEFAULT_COLS, DEFAULT_ROWS};

const CORRUPT_ARCHIVE_COLUMN_SQL: &str = "UPDATE sessions SET archived = ?1 WHERE session_id = ?2";
const CORRUPT_UPDATED_AT_COLUMN_SQL: &str =
    "UPDATE sessions SET updated_at_ms = ?1 WHERE session_id = ?2";
const REPLACE_MANIFEST_JSON_SQL: &str =
    "UPDATE sessions SET manifest_json = ?1 WHERE session_id = ?2";
const DIFFERENT_UPDATED_AT_MS: i64 = 1;
const TEST_NAME: &str = "archive-column-fence";
const TEST_COMMAND: &str = "/bin/sh";
const TEST_ARGUMENT: &str = "-l";

#[test]
fn active_listing_rejects_a_manifest_archive_column_mismatch() {
    let directory = tempfile::tempdir().unwrap();
    let mut catalog = Catalog::open(&directory.path().join("catalog.sqlite")).unwrap();
    let manifest = active_manifest();
    catalog.insert(&manifest).unwrap();
    catalog
        .connection
        .execute(
            CORRUPT_ARCHIVE_COLUMN_SQL,
            rusqlite::params![constants::ARCHIVED_TRUE, manifest.terminal_id.to_string()],
        )
        .unwrap();

    let listing = catalog.list_tolerant(false).unwrap();
    assert!(listing.manifests.is_empty());
    assert_eq!(listing.diagnostics.len(), 1);
    assert_eq!(listing.diagnostics[0].terminal_id, Some(manifest.terminal_id));
    assert!(catalog.list(false).is_err());
}

#[test]
fn complete_listing_rejects_every_noncanonical_row_representation() {
    let directory = tempfile::tempdir().unwrap();
    let mut catalog = Catalog::open(&directory.path().join("catalog.sqlite")).unwrap();
    let manifest = active_manifest();
    catalog.insert(&manifest).unwrap();
    catalog
        .connection
        .execute(
            CORRUPT_UPDATED_AT_COLUMN_SQL,
            rusqlite::params![DIFFERENT_UPDATED_AT_MS, manifest.terminal_id.to_string()],
        )
        .unwrap();
    assert!(catalog.list(false).is_err());

    catalog.upsert(&manifest).unwrap();
    let noncanonical = serde_json::to_vec_pretty(&manifest).unwrap();
    catalog
        .connection
        .execute(
            REPLACE_MANIFEST_JSON_SQL,
            rusqlite::params![noncanonical, manifest.terminal_id.to_string()],
        )
        .unwrap();
    assert!(catalog.list(false).is_err());
}

fn active_manifest() -> TerminalManifest {
    TerminalManifest {
        schema_version: SESSION_MANIFEST_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        name: TEST_NAME.into(),
        command: TEST_COMMAND.into(),
        args: vec![TEST_ARGUMENT.into()],
        cwd: std::env::current_dir().unwrap(),
        cols: DEFAULT_COLS,
        rows: DEFAULT_ROWS,
        created_at_ms: now_ms(),
        updated_at_ms: now_ms(),
        archived: false,
        recovery_state: RecoveryState::Created,
    }
}
