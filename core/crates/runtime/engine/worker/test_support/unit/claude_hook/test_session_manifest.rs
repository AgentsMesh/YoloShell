use crate::paths::Paths;
use uuid::Uuid;

const TEST_COMMAND: &str = "/bin/sh";
const TEST_SESSION_NAME: &str = "claude-hook-test";

pub(super) fn ensure(paths: &Paths, terminal_id: Uuid) {
    paths.ensure_session(terminal_id).unwrap();
    let timestamp = crate::model::now_ms();
    let manifest = crate::model::TerminalManifest {
        schema_version: crate::constants::SESSION_MANIFEST_SCHEMA_VERSION,
        terminal_id: terminal_id,
        name: TEST_SESSION_NAME.to_owned(),
        command: TEST_COMMAND.to_owned(),
        args: Vec::new(),
        cwd: paths.home().to_owned(),
        cols: crate::model::DEFAULT_COLS,
        rows: crate::model::DEFAULT_ROWS,
        created_at_ms: timestamp,
        updated_at_ms: timestamp,
        archived: false,
        recovery_state: crate::model::RecoveryState::Created,
    };
    crate::paths::atomic_write_json(&paths.manifest(terminal_id), &manifest).unwrap();
}
