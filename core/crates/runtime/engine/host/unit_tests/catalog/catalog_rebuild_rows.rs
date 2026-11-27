use crate::host::tests::*;
use super::*;

const CATALOG_SESSION_ID_COLUMN: usize = 0;
const CATALOG_MANIFEST_JSON_COLUMN: usize = 1;
const CATALOG_UPDATED_AT_COLUMN: usize = 2;
const CATALOG_ARCHIVED_COLUMN: usize = 3;
const UPDATE_ROW_ID_SQL: &str = "UPDATE sessions SET session_id = ?1 WHERE session_id = ?2";
const UPDATE_ROW_JSON_SQL: &str = "UPDATE sessions SET manifest_json = ?1 WHERE session_id = ?2";
const UPDATE_ROW_TIME_SQL: &str = "UPDATE sessions SET updated_at_ms = ?1 WHERE session_id = ?2";
const UPDATE_ROW_ARCHIVE_SQL: &str = "UPDATE sessions SET archived = ?1 WHERE session_id = ?2";
const SELECT_RAW_ROW_SQL: &str = "SELECT session_id, manifest_json, updated_at_ms, archived \
    FROM sessions WHERE session_id = ?1";
const OLDER_UPDATED_AT_MS: i64 = 100;
const NEWER_UPDATED_AT_MS: i64 = 200;
const DIVERGENT_UPDATED_AT_MS: i64 = 300;
const ARCHIVED_VALUE: i64 = 1;

#[test]
fn raw_catalog_row_divergence_is_rebuilt_and_canonical_apis_remain_consistent() {
    let directory = tempdir().unwrap();
    let mut older = fixture_manifest(directory.path());
    older.updated_at_ms = OLDER_UPDATED_AT_MS;
    let mut newer = fixture_manifest(directory.path());
    newer.updated_at_ms = NEWER_UPDATED_AT_MS;
    persist_exact_manifest(directory.path(), &older);
    persist_exact_manifest(directory.path(), &newer);
    drop(Host::open(HostConfig::new(directory.path())).unwrap());
    let catalog_path = Paths::new(directory.path()).catalog();

    rusqlite::Connection::open(&catalog_path)
        .unwrap()
        .execute(
            UPDATE_ROW_ID_SQL,
            rusqlite::params![
                older.terminal_id.to_string().to_uppercase(),
                older.terminal_id.to_string()
            ],
        )
        .unwrap();
    assert_catalog_repaired(directory.path(), &[&older, &newer]);

    rusqlite::Connection::open(&catalog_path)
        .unwrap()
        .execute(
            UPDATE_ROW_JSON_SQL,
            rusqlite::params![
                serde_json::to_vec_pretty(&older).unwrap(),
                older.terminal_id.to_string()
            ],
        )
        .unwrap();
    assert_catalog_repaired(directory.path(), &[&older, &newer]);

    rusqlite::Connection::open(&catalog_path)
        .unwrap()
        .execute(
            UPDATE_ROW_TIME_SQL,
            rusqlite::params![DIVERGENT_UPDATED_AT_MS, older.terminal_id.to_string()],
        )
        .unwrap();
    assert_catalog_repaired(directory.path(), &[&older, &newer]);

    rusqlite::Connection::open(&catalog_path)
        .unwrap()
        .execute(
            UPDATE_ROW_ARCHIVE_SQL,
            rusqlite::params![ARCHIVED_VALUE, older.terminal_id.to_string()],
        )
        .unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    {
        let mut catalog = lock_recover(&host.catalog);
        let active = catalog.list(false).unwrap();
        assert_eq!(active.len(), 2);
        assert_eq!(active[0].terminal_id, newer.terminal_id);
        assert_eq!(active[1].terminal_id, older.terminal_id);
        assert!(
            catalog
                .archive(older.terminal_id, true)
                .unwrap()
                .unwrap()
                .archived
        );
        assert_eq!(catalog.list(false).unwrap().len(), 1);
        let updated = catalog
            .update_recovery(newer.terminal_id, RecoveryState::Resumed)
            .unwrap()
            .unwrap();
        assert_eq!(updated.recovery_state, RecoveryState::Resumed);
        assert_eq!(
            catalog
                .get(newer.terminal_id)
                .unwrap()
                .unwrap()
                .recovery_state,
            RecoveryState::Resumed
        );
    }
    drop(host);

    assert_raw_row_is_canonical(&catalog_path, older.terminal_id);
    assert_raw_row_is_canonical(&catalog_path, newer.terminal_id);
}

fn persist_exact_manifest(home: &Path, manifest: &TerminalManifest) {
    let paths = Paths::new(home);
    paths.ensure_session(manifest.terminal_id).unwrap();
    atomic_write_json(&paths.manifest(manifest.terminal_id), manifest).unwrap();
}

fn assert_catalog_repaired(home: &Path, manifests: &[&TerminalManifest]) {
    let host = Host::open(HostConfig::new(home)).unwrap();
    let catalog = lock_recover(&host.catalog);
    for manifest in manifests {
        assert_eq!(
            serde_json::to_vec(&catalog.get(manifest.terminal_id).unwrap().unwrap()).unwrap(),
            serde_json::to_vec(manifest).unwrap()
        );
    }
}

fn assert_raw_row_is_canonical(path: &Path, terminal_id: Uuid) {
    let connection = rusqlite::Connection::open(path).unwrap();
    let (key, bytes, updated_at_ms, archived): (String, Vec<u8>, i64, i64) = connection
        .query_row(SELECT_RAW_ROW_SQL, [terminal_id.to_string()], |row| {
            Ok((
                row.get(CATALOG_SESSION_ID_COLUMN)?,
                row.get(CATALOG_MANIFEST_JSON_COLUMN)?,
                row.get(CATALOG_UPDATED_AT_COLUMN)?,
                row.get(CATALOG_ARCHIVED_COLUMN)?,
            ))
        })
        .unwrap();
    let manifest: TerminalManifest = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(key, manifest.terminal_id.to_string());
    assert_eq!(bytes, serde_json::to_vec(&manifest).unwrap());
    assert_eq!(updated_at_ms, manifest.updated_at_ms);
    assert_eq!(archived, manifest.archived as i64);
}
