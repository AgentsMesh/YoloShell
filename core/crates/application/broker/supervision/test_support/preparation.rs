pub use crate::fixture_constants::*;
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

pub struct RuntimeFixture {
    _root: tempfile::TempDir,
    pub config: PathBuf,
    pub state_dir: PathBuf,
}

impl RuntimeFixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let known_hosts = root.path().join(KNOWN_HOSTS_FILE);
        fs::write(&known_hosts, KNOWN_HOST).unwrap();
        fs::set_permissions(&known_hosts, fs::Permissions::from_mode(TRUSTED_FILE_MODE)).unwrap();
        let document = serde_json::json!({
            (VERSION_FIELD): CONFIG_VERSION,
            (HOST_ID_FIELD): TEST_HOST_ID,
            (PROFILE_AUTHORITY_SHA256_FIELD): TEST_PROFILE_AUTHORITY_SHA256,
            (ENDPOINT_FIELD): {(USER_FIELD): TEST_USER, (HOST_FIELD): TEST_HOST, (PORT_FIELD): TEST_PORT},
            (IDENTITIES_FIELD): [], (USE_AGENT_FIELD): false,
            (RUNTIME_FIELD): {(EXECUTABLE_FIELD): TEST_RUNTIME, (HOME_FIELD): TEST_HOME},
            (POLICY_FIELD): {(ALLOWED_PROGRAMS_FIELD): [TEST_PROGRAM], (READ_ROOTS_FIELD): [TEST_READ_ROOT]},
            (KNOWN_HOSTS_FIELD): known_hosts,
            (HUB_TRANSPORT_FIELD): false,
        });
        let config = root.path().join(CONFIG_FILE);
        fs::write(&config, serde_json::to_vec(&document).unwrap()).unwrap();
        fs::set_permissions(&config, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
        Self {
            config,
            state_dir: root.path().join(STATE_DIRECTORY),
            _root: root,
        }
    }
}
