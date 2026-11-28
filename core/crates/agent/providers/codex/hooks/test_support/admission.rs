#![allow(dead_code)]

use crate::agent::hooks::{Generation, HookIdentity, ProcessIdentity};
use crate::codex_hooks::{
    CodexAdmissionContext, CodexAdmissionRecord, CodexAdmissionStorage, CodexHookArtifacts,
};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use uuid::Uuid;

pub(super) const EXECUTABLE_CONTENT: &[u8] = b"#!/bin/sh\nexit 0\n";
pub(super) const CHANGED_EXECUTABLE_CONTENT: &[u8] = b"#!/bin/sh\nexit 1\n";
pub(super) const EXECUTABLE_MODE: u32 = crate::constants::RUNTIME_EXECUTABLE_MODE;
pub(super) const NON_EXECUTABLE_MODE: u32 = 0o600;
pub(super) const UNSAFE_EXECUTABLE_MODE: u32 = 0o722;
pub(super) const TEST_PROCESS_PID: u32 = 101;
pub(super) const TEST_PROCESS_START: u64 = 103;
pub(super) const UNKNOWN_FIELD: &str = "unexpected";
pub(super) const UNKNOWN_VALUE: bool = true;
const HOME_DIRECTORY: &str = "home";
const EXECUTABLE_DIRECTORY: &str = "executables";
const CWD_DIRECTORY: &str = "project";
const DATA_HOME_DIRECTORY: &str = "provider-home";
const HOOK_FILE_NAME: &str = "codex-hook";
const REAL_FILE_NAME: &str = "codex-real";

pub(super) struct Fixture {
    _root: TempDir,
    pub(super) paths: crate::paths::Paths,
    pub(super) identity: HookIdentity,
    pub(super) artifacts: CodexHookArtifacts,
    pub(super) request: CodexAdmissionContext,
}

impl Fixture {
    pub(super) fn new() -> Self {
        Self::build(None)
    }

    pub(super) fn with_identity(identity: HookIdentity) -> Self {
        Self::build(Some(identity))
    }

    fn build(identity: Option<HookIdentity>) -> Self {
        let root = tempfile::tempdir().unwrap();
        let canonical_root = root.path().canonicalize().unwrap();
        let paths = crate::paths::Paths::new(canonical_root.join(HOME_DIRECTORY));
        let identity = identity.unwrap_or_else(|| HookIdentity {
            terminal_id: Uuid::new_v4(),
            runtime_epoch: Uuid::new_v4(),
            invocation_id: Uuid::new_v4(),
            hook_token: Uuid::new_v4(),
            process: ProcessIdentity::new(TEST_PROCESS_PID, TEST_PROCESS_START),
            generation: Generation::INITIAL,
        });
        paths.ensure_session(identity.terminal_id).unwrap();
        let executable_directory = canonical_root.join(EXECUTABLE_DIRECTORY);
        std::fs::create_dir(&executable_directory).unwrap();
        let hook = executable(
            &executable_directory.join(HOOK_FILE_NAME),
            EXECUTABLE_CONTENT,
        );
        let real = executable(
            &executable_directory.join(REAL_FILE_NAME),
            EXECUTABLE_CONTENT,
        );
        let cwd = canonical_root.join(CWD_DIRECTORY);
        let data_home = canonical_root.join(DATA_HOME_DIRECTORY);
        std::fs::create_dir(&cwd).unwrap();
        std::fs::create_dir(&data_home).unwrap();
        let request = CodexAdmissionContext {
            identity: identity.clone(),
            provider_version: crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
            cwd,
            data_home,
            hook_executable: hook,
            real_executable: real,
        };
        let artifacts = CodexHookArtifacts::new(
            &paths,
            identity.terminal_id,
            identity.runtime_epoch,
            identity.invocation_id,
        )
        .unwrap();
        Self {
            _root: root,
            paths,
            identity,
            artifacts,
            request,
        }
    }

    pub(super) fn record(&self) -> CodexAdmissionRecord {
        CodexAdmissionRecord::capture(&self.request).unwrap()
    }

    pub(super) fn storage(&self) -> CodexAdmissionStorage {
        CodexAdmissionStorage::for_identity(&self.artifacts, &self.identity).unwrap()
    }

    pub(super) fn alternate_executable(&self, name: &str, contents: &[u8]) -> PathBuf {
        executable(
            &self.request.hook_executable.parent().unwrap().join(name),
            contents,
        )
    }

    pub(super) fn replace_hook(&self, contents: &[u8]) {
        let replacement = self.alternate_executable(UNKNOWN_FIELD, contents);
        std::fs::rename(replacement, &self.request.hook_executable).unwrap();
    }
}

pub(super) fn executable(path: &Path, contents: &[u8]) -> PathBuf {
    std::fs::write(path, contents).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(EXECUTABLE_MODE)).unwrap();
    path.to_path_buf()
}
