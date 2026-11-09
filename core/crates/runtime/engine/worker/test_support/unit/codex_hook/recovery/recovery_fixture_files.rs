use super::support::Fixture;
use crate::agent::hooks::{Generation, HookIdentity, ProcessIdentity};
use crate::codex_hooks::CodexAdmissionContext;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const EXECUTABLE_DIRECTORY: &str = "recovery-executables";
const CWD_DIRECTORY: &str = "recovery-project";
const DATA_HOME_DIRECTORY: &str = "recovery-data-home";
const HOOK_FILE_NAME: &str = "codex-hook";
const EXECUTABLE_CONTENT: &[u8] = b"#!/bin/sh\nexit 0\n";

pub(super) fn identity(process: crate::process::ProcessIdentity) -> HookIdentity {
    HookIdentity {
        terminal_id: Fixture::terminal_id(),
        runtime_epoch: Fixture::runtime_epoch(),
        invocation_id: Uuid::new_v4(),
        hook_token: Uuid::new_v4(),
        process: ProcessIdentity::new(process.pid, process.start_time),
        generation: Generation::INITIAL,
    }
}

pub(super) fn request(root: &Path, identity: HookIdentity) -> CodexAdmissionContext {
    let executables = directory(&root.join(EXECUTABLE_DIRECTORY));
    let real = observed_executable(identity.process.pid);
    CodexAdmissionContext::new(
        identity,
        directory(&root.join(CWD_DIRECTORY)),
        directory(&root.join(DATA_HOME_DIRECTORY)),
        executable(&executables.join(HOOK_FILE_NAME)),
        real,
    )
}

fn observed_executable(_pid: u32) -> PathBuf {
    #[cfg(target_os = "macos")]
    if let Ok(path) = crate::process::macos_native::executable_path(_pid) {
        return path.canonicalize().unwrap();
    }
    std::env::current_exe().unwrap().canonicalize().unwrap()
}

fn directory(path: &Path) -> PathBuf {
    std::fs::create_dir(path).unwrap();
    path.to_path_buf()
}

fn executable(path: &Path) -> PathBuf {
    std::fs::write(path, EXECUTABLE_CONTENT).unwrap();
    std::fs::set_permissions(
        path,
        std::fs::Permissions::from_mode(crate::constants::RUNTIME_EXECUTABLE_MODE),
    )
    .unwrap();
    path.to_path_buf()
}
