use super::*;
use crate::host::tests::*;

pub(super) fn ledger_test_spec(directory: &Path) -> CreationSpec {
    CreationSpec {
        request_fingerprint: yolo_crypto::sha256_hex(b"request"),
        name: Some("ledger-test".into()),
        cwd: directory.to_path_buf(),
        cols: None,
        rows: None,
        initial_input: None,
        context: None,
    }
}

pub(super) fn ledger_test_binding(directory: &Path) -> CreationBinding {
    CreationBinding {
        creation_token_digest: yolo_crypto::sha256_hex(b"token"),
        result: CreateTerminalResult {
            terminal_id: Uuid::new_v4(),
            creation_capability: Uuid::new_v4(),
        },
        spec: ledger_test_spec(directory),
        phase: CreationPhase::Ready,
        launch_generation: INITIAL_CREATION_LAUNCH_GENERATION,
        launch: Some(CreationLaunchIdentity::new()),
    }
}

pub(super) fn set_private_file_mode(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
}

