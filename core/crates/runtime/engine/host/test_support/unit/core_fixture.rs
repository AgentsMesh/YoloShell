const TEST_REQUEST_ID: &str = "test-request";
const TEST_SESSION_NAME: &str = "fixture";
const TEST_TERMINAL_COLUMNS: u16 = 80;
const TEST_TERMINAL_ROWS: u16 = 24;

fn fixture_manifest(path: &Path) -> TerminalManifest {
    let id = Uuid::new_v4();
    TerminalManifest {
        schema_version: SESSION_MANIFEST_SCHEMA_VERSION,
        terminal_id: id,
        name: TEST_SESSION_NAME.into(),
        command: crate::agent::constants::FALLBACK_SHELL.into(),
        args: Vec::new(),
        cwd: path.to_path_buf(),
        cols: TEST_TERMINAL_COLUMNS,
        rows: TEST_TERMINAL_ROWS,
        created_at_ms: now_ms(),
        updated_at_ms: now_ms(),
        archived: false,
        recovery_state: RecoveryState::Created,
    }
}

fn request<T: Serialize>(method_name: &str, payload: &T) -> RequestEnvelope {
    RequestEnvelope::new(TEST_REQUEST_ID, method_name, to_value(payload).unwrap())
}

fn write_private_bytes(path: &Path, bytes: &[u8]) {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let mode = crate::paths::constants::PRIVATE_FILE_MODE;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(mode)
        .open(path)
        .unwrap();
    file.set_permissions(std::fs::Permissions::from_mode(mode))
        .unwrap();
    file.write_all(bytes).unwrap();
}
