use uuid::Uuid;

const SESSION_NAME: &str = "managed-begin-availability";

pub(super) fn write(
    paths: &crate::paths::Paths,
    session_id: Uuid,
    cwd: &std::path::Path,
) {
    let timestamp = crate::model::now_ms();
    let manifest = crate::model::TerminalManifest {
        schema_version: crate::runtime_constants::SESSION_MANIFEST_SCHEMA_VERSION,
        terminal_id: session_id,
        name: SESSION_NAME.into(),
        command: crate::agent::constants::FALLBACK_SHELL.into(),
        args: Vec::new(),
        cwd: cwd.to_owned(),
        cols: crate::model::DEFAULT_COLS,
        rows: crate::model::DEFAULT_ROWS,
        created_at_ms: timestamp,
        updated_at_ms: timestamp,
        archived: false,
        recovery_state: crate::model::RecoveryState::Created,
    };
    crate::paths::atomic_write_json(&paths.manifest(session_id), &manifest).unwrap();
}
