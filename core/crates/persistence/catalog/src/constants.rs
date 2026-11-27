use crate::model::TerminalManifest;
use std::time::Duration;

pub(super) const ARCHIVED_FALSE: i64 = 0;
pub(super) const ARCHIVED_TRUE: i64 = 1;
pub(crate) const ARCHIVED_COLUMN: &str = "archived";
pub(super) fn archived_value(archived: bool) -> i64 {
    if archived {
        ARCHIVED_TRUE
    } else {
        ARCHIVED_FALSE
    }
}

pub(super) fn catalog_schema_sql() -> String {
    format!(
        "CREATE TABLE IF NOT EXISTS metadata (
       key TEXT PRIMARY KEY NOT NULL,
       value TEXT NOT NULL
     );
     CREATE TABLE IF NOT EXISTS sessions (
       session_id TEXT PRIMARY KEY NOT NULL,
       manifest_json BLOB NOT NULL,
       updated_at_ms INTEGER NOT NULL,
       archived INTEGER NOT NULL DEFAULT {}
         CHECK (archived IN ({}, {}))
     );
     CREATE INDEX IF NOT EXISTS sessions_archived_updated
       ON sessions(archived, updated_at_ms DESC);",
        ARCHIVED_FALSE, ARCHIVED_FALSE, ARCHIVED_TRUE
    )
}
pub(super) const CONTROLLER_GENERATION_KEY: &str = "controller_generation";
pub(crate) const CONTROLLER_GENERATION_EXHAUSTED: &str = "controller generation exhausted";
pub(super) const FOREIGN_KEYS_PRAGMA: &str = "foreign_keys";
pub(super) const FOREIGN_KEYS_VALUE: &str = "ON";
pub(super) const INITIAL_CONTROLLER_GENERATION: u64 = 0;
pub(super) const INSERT_METADATA_SQL: &str =
    "INSERT INTO metadata(key, value) VALUES(?1, ?2) ON CONFLICT(key) DO NOTHING";
pub(super) const INTEGRITY_CHECK_SQL: &str = "PRAGMA quick_check";
pub(super) const INTEGRITY_OK: &str = "ok";
pub(super) const JOURNAL_MODE_PRAGMA: &str = "journal_mode";
pub(super) const JOURNAL_MODE_VALUE: &str = "WAL";
pub(crate) const MANIFEST_JSON_COLUMN: &str = "manifest_json";
pub(super) const METADATA_VALUE_COLUMN: &str = "value";
pub(super) const MAX_SECURE_VFS_PARENT_ENTRIES: usize = 4_096;
pub(super) const MAX_CATALOG_ARTIFACT_BYTES: u64 = u64::MAX;
pub(super) const MIN_ALIAS_OWNER_PROCESS_ID: libc::pid_t = 1;
pub(super) const READ_METADATA_SQL: &str = "SELECT value FROM metadata WHERE key = ?1";
pub(super) const READ_SESSION_SQL: &str =
    "SELECT manifest_json FROM sessions WHERE session_id = ?1";
pub(crate) const SESSION_ID_COLUMN: &str = "session_id";
pub(crate) const UPDATED_AT_COLUMN: &str = "updated_at_ms";
pub(super) const UNREADABLE_ROW_KEY: &str = "<unreadable>";
pub(super) const QUICK_CHECK_COLUMN: &str = "quick_check";
pub(super) const DELETE_SESSION_SQL: &str = "DELETE FROM sessions WHERE session_id = ?1";
pub(super) const INSERT_SESSION_SQL: &str = "INSERT INTO sessions(session_id, manifest_json, updated_at_ms, archived) \
     VALUES(?1, ?2, ?3, ?4)";
pub(super) const UPSERT_SESSION_SQL: &str = "INSERT INTO sessions(session_id, manifest_json, updated_at_ms, archived) \
     VALUES(?1, ?2, ?3, ?4) \
     ON CONFLICT(session_id) DO UPDATE SET \
       manifest_json = excluded.manifest_json, \
       updated_at_ms = excluded.updated_at_ms, \
       archived = excluded.archived";
pub(super) const SCHEMA_VERSION: i64 = 1;
pub(super) const SCHEMA_VERSION_KEY: &str = "schema_version";
pub(super) const CATALOG_QUARANTINE_MARKER: &str = ".quarantine-";
pub(super) const ALIAS_MAIN_NAME: &str = "catalog";
pub(super) const SQLITE_JOURNAL_SUFFIX: &str = "-journal";
pub(super) const SQLITE_SHM_SUFFIX: &str = "-shm";
pub(super) const SQLITE_WAL_SUFFIX: &str = "-wal";
pub(super) const SQLITE_BUSY_TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const SYNCHRONOUS_PRAGMA: &str = "synchronous";
pub(super) const SYNCHRONOUS_VALUE: &str = "FULL";

pub(super) fn invalid_catalog_session_detail(row_key: &str, detail: &str) -> String {
    format!("catalog session {row_key} is invalid: {detail}")
}

pub(super) fn inconsistent_manifest_row_detail(
    manifest: &TerminalManifest,
    updated_at_ms: i64,
    archived: i64,
) -> String {
    format!(
        "manifest row is inconsistent: id={}, schema={}, updated_at_ms={updated_at_ms}, manifest_updated_at_ms={}, archived={archived}, manifest_archived={}",
        manifest.terminal_id, manifest.schema_version, manifest.updated_at_ms, manifest.archived
    )
}

pub(super) fn unsupported_manifest_schema_detail(schema_version: u16) -> String {
    format!("unsupported session manifest schema {schema_version}")
}
