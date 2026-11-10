use std::path::{Path, PathBuf};
use uuid::Uuid;

const HOME_DIRECTORY: &str = "home";
const PROJECT_DIRECTORY: &str = "project";
const SESSION_NAME: &str = "claude-generation-test";

pub(super) struct ManagedSession {
    pub data_home: PathBuf,
    pub cwd: PathBuf,
    pub paths: crate::paths::Paths,
    pub session_id: Uuid,
}

pub(super) fn prepare(root: &Path) -> ManagedSession {
    let root = root.canonicalize().unwrap();
    let paths = crate::paths::Paths::new(root.join(HOME_DIRECTORY));
    let session_id = Uuid::new_v4();
    paths.ensure_session(session_id).unwrap();
    let data_home = root.join(crate::agent::claude::constants::DEFAULT_DATA_HOME_DIR);
    let cwd = root.join(PROJECT_DIRECTORY);
    std::fs::create_dir_all(&data_home).unwrap();
    std::fs::create_dir_all(&cwd).unwrap();
    write_manifest(&paths, session_id, &cwd);
    ManagedSession {
        data_home,
        cwd,
        paths,
        session_id,
    }
}

fn write_manifest(paths: &crate::paths::Paths, session_id: Uuid, cwd: &Path) {
    let timestamp = crate::model::now_ms();
    let manifest = crate::model::TerminalManifest {
        schema_version: crate::process::runtime_constants::SESSION_MANIFEST_SCHEMA_VERSION,
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
