use super::*;

const PRIMARY_GUARDIAN_OFFSET: u32 = 2;
const STANDBY_GUARDIAN_OFFSET: u32 = 3;
const TARGET_OFFSET: u32 = 0;
pub(super) const FOREIGN_SETTINGS_DIRECTORY: &str = "foreign-settings";
pub(super) const TEST_HOME_DIRECTORY: &str = "home";
const TEST_COMMAND: &str = "/bin/sh";
const TEST_NAME: &str = "custody-fence";
const TEST_PID: u32 = 950_001;
const TEST_START_TIME: u64 = 960_001;
const TEST_TERMINAL_COLUMNS: u16 = 120;
const TEST_TERMINAL_ROWS: u16 = 32;
const WORKER_OFFSET: u32 = 1;

pub(super) struct Fixture {
    pub(super) _directory: tempfile::TempDir,
    pub(super) paths: crate::paths::Paths,
    pub(super) terminal_id: uuid::Uuid,
    pub(super) ledger: std::path::PathBuf,
    pub(super) spec: CustodySpec,
}

impl Fixture {
    pub(super) fn new(archived: bool, manifest_session: Option<uuid::Uuid>) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let home = directory.path().join(TEST_HOME_DIRECTORY);
        let paths = crate::paths::Paths::new(home);
        let terminal_id = uuid::Uuid::new_v4();
        paths.ensure_session(terminal_id).unwrap();
        write_manifest(
            &paths,
            terminal_id,
            manifest_session.unwrap_or(terminal_id),
            archived,
        );
        let settings = paths
            .session_dir(terminal_id)
            .join(crate::claude_settings_artifact::DIRECTORY_NAME);
        crate::paths::directory::PrivateDirectory::ensure(&settings).unwrap();
        Self {
            _directory: directory,
            ledger: path(&settings).unwrap(),
            spec: custody_spec(),
            paths,
            terminal_id,
        }
    }
}

pub(super) fn publish_intent_marker(fixture: &Fixture) {
    crate::paths::atomic_write_json(
        &fixture.paths.reboot_migration_intent(fixture.terminal_id),
        &serde_json::Value::Null,
    )
    .unwrap();
}

fn custody_spec() -> CustodySpec {
    let identity = |offset: u32| DurableProcessIdentity {
        boot_id: crate::process::boot_id(),
        pid: TEST_PID + offset,
        start_time: TEST_START_TIME + u64::from(offset),
    };
    CustodySpec {
        invocation_id: uuid::Uuid::new_v4(),
        target: identity(TARGET_OFFSET),
        worker: identity(WORKER_OFFSET),
        guardians: [
            identity(PRIMARY_GUARDIAN_OFFSET),
            identity(STANDBY_GUARDIAN_OFFSET),
        ],
    }
}

fn write_manifest(
    paths: &crate::paths::Paths,
    directory_session: uuid::Uuid,
    manifest_session: uuid::Uuid,
    archived: bool,
) {
    let timestamp = crate::model::now_ms();
    let manifest = crate::model::TerminalManifest {
        schema_version: crate::constants::SESSION_MANIFEST_SCHEMA_VERSION,
        terminal_id: manifest_session,
        name: TEST_NAME.into(),
        command: TEST_COMMAND.into(),
        args: Vec::new(),
        cwd: paths.home().to_owned(),
        cols: TEST_TERMINAL_COLUMNS,
        rows: TEST_TERMINAL_ROWS,
        created_at_ms: timestamp,
        updated_at_ms: timestamp,
        archived,
        recovery_state: crate::model::RecoveryState::Created,
    };
    crate::paths::atomic_write_json(&paths.manifest(directory_session), &manifest).unwrap();
}
