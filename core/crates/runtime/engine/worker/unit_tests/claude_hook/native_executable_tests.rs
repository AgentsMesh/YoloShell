use super::*;
use std::os::unix::fs::{PermissionsExt, symlink};

const FIXTURE_NAME: &str = "native-hook";
const LINK_NAME: &str = "native-hook-link";
const EXECUTABLE_MODE: u32 = 0o700;
const READ_ONLY_MODE: u32 = 0o600;
const ORIGINAL: &[u8] = b"native-hook-first-object";
const REPLACEMENT: &[u8] = b"native-hook-other-object";

#[test]
fn current_mapped_executable_has_native_callback_authority() {
    let executable = std::env::current_exe().unwrap().canonicalize().unwrap();
    let authority = HookExecutable::open(executable).unwrap();
    authority.verify_process(std::process::id()).unwrap();
}

#[test]
fn unrelated_executable_inode_cannot_claim_current_process() {
    let directory = tempfile::tempdir().unwrap();
    let executable = directory.path().join(FIXTURE_NAME);
    std::fs::write(&executable, ORIGINAL).unwrap();
    std::fs::set_permissions(
        &executable,
        std::fs::Permissions::from_mode(EXECUTABLE_MODE),
    )
    .unwrap();
    let authority = HookExecutable::open(executable.canonicalize().unwrap()).unwrap();
    assert!(authority.verify_process(std::process::id()).is_err());
    std::fs::write(&executable, REPLACEMENT).unwrap();
    assert!(authority.verify().is_err());
}

#[test]
fn symlink_and_non_executable_entries_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let executable = directory.path().join(FIXTURE_NAME);
    std::fs::write(&executable, ORIGINAL).unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(READ_ONLY_MODE)).unwrap();
    assert!(HookExecutable::open(executable.canonicalize().unwrap()).is_err());
    let link = directory.path().join(LINK_NAME);
    symlink(&executable, &link).unwrap();
    assert!(HookExecutable::open(link).is_err());
}
