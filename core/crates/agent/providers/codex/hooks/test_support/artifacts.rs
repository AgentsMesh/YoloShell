use super::*;
use crate::codex_hooks::constants::test_values;
use tempfile::TempDir;

const TEST_PROCESS_PID: u32 = 51;
const TEST_PROCESS_START: u64 = 61;

pub(super) struct Fixture {
    pub(super) root: TempDir,
    pub(super) paths: crate::paths::Paths,
    pub(super) session_id: Uuid,
    pub(super) runtime_epoch: Uuid,
    pub(super) invocation_id: Uuid,
    pub(super) artifacts: CodexHookArtifacts,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let paths = crate::paths::Paths::new(root.path().join(test_values::HOME_DIRECTORY));
        let session_id = Uuid::new_v4();
        let runtime_epoch = Uuid::new_v4();
        let invocation_id = Uuid::new_v4();
        let artifacts =
            CodexHookArtifacts::new(&paths, session_id, runtime_epoch, invocation_id).unwrap();
        Self {
            root,
            paths,
            session_id,
            runtime_epoch,
            invocation_id,
            artifacts,
        }
    }

    pub(super) fn identity(&self) -> crate::agent::hooks::HookIdentity {
        crate::agent::hooks::HookIdentity {
            terminal_id: self.session_id,
            runtime_epoch: self.runtime_epoch,
            invocation_id: self.invocation_id,
            hook_token: Uuid::new_v4(),
            process: crate::agent::hooks::ProcessIdentity::new(
                TEST_PROCESS_PID,
                TEST_PROCESS_START,
            ),
            generation: crate::agent::hooks::Generation::INITIAL,
        }
    }
}

#[cfg(unix)]
pub(super) fn private_directory(path: &std::path::Path) {
    crate::paths::directory::PrivateDirectory::ensure(path).unwrap();
}

#[cfg(unix)]
pub(super) fn assert_outbox_rejected(fixture: &Fixture) {
    assert!(
        CodexOutbox::for_identity(fixture.artifacts.outbox_path(), &fixture.identity()).is_err()
    );
}
