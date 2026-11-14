const TEST_COMMAND: &str = "/bin/sh";
const TEST_SESSION_NAME: &str = "claude-custody-fixture";

#[doc(hidden)]
pub fn prepare_managed_settings_for_test(root: &Path) -> Result<PathBuf> {
    let canonical_root = root.canonicalize()?;
    let paths = crate::paths::Paths::new(canonical_root.join("home"));
    let terminal_id = uuid::Uuid::new_v4();
    paths.ensure_session(terminal_id)?;
    let timestamp = crate::model::now_ms();
    let manifest = crate::model::TerminalManifest {
        schema_version: crate::constants::SESSION_MANIFEST_SCHEMA_VERSION,
        terminal_id: terminal_id,
        name: TEST_SESSION_NAME.into(),
        command: TEST_COMMAND.into(),
        args: Vec::new(),
        cwd: canonical_root,
        cols: crate::model::DEFAULT_COLS,
        rows: crate::model::DEFAULT_ROWS,
        created_at_ms: timestamp,
        updated_at_ms: timestamp,
        archived: false,
        recovery_state: crate::model::RecoveryState::Created,
    };
    crate::paths::atomic_write_json(&paths.manifest(terminal_id), &manifest)?;
    let settings = paths
        .session_dir(terminal_id)
        .join(crate::claude_settings_artifact::DIRECTORY_NAME);
    crate::paths::directory::PrivateDirectory::ensure(&settings)?;
    Ok(settings)
}
