use super::admission_test_support::Fixture;
use super::{CodexAdmissionAuthority, CodexHookArtifacts, scan_admissions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub(super) const STAGING_BYTES: &[u8] = b"pre-commit admission";
pub(super) const REPLACEMENT_BYTES: &[u8] = b"replacement";

pub(super) struct StagedOrphan {
    pub(super) artifacts: CodexHookArtifacts,
    pub(super) staging_path: PathBuf,
    pub(super) target_path: PathBuf,
}

pub(super) fn stage(fixture: &Fixture, invocation: u128) -> StagedOrphan {
    let artifacts = artifacts(fixture, invocation);
    let directory =
        crate::paths::directory::PrivateDirectory::ensure(artifacts.directory()).unwrap();
    create_writer_lock(&directory, &artifacts);
    directory
        .ensure_descendant(&artifacts.intake_directory())
        .unwrap();
    let target_path = artifacts
        .directory()
        .join(super::admission_storage::CODEX_ADMISSION_FILE_NAME);
    let target = crate::paths::private_file::PrivateFileEntry::from_directory(
        directory.try_clone().unwrap(),
        &target_path,
    )
    .unwrap();
    let staging = target.exclusive_create_staging_entry().unwrap();
    crate::paths::test_support::write_private_staging(
        &staging,
        STAGING_BYTES,
        crate::paths::constants::MAX_DURABLE_JSON_BYTES,
    )
    .unwrap();
    StagedOrphan {
        artifacts,
        staging_path: staging.path().to_path_buf(),
        target_path,
    }
}

pub(super) fn scan(fixture: &Fixture) -> anyhow::Result<Vec<CodexAdmissionAuthority>> {
    scan_admissions(
        &fixture.paths,
        fixture.identity.terminal_id,
        fixture.identity.runtime_epoch,
    )
}

pub(super) fn acquire_writer(
    staged: &StagedOrphan,
) -> crate::paths::private_file_lock::PrivateFileLock {
    let directory =
        crate::paths::directory::PrivateDirectory::open(staged.artifacts.directory()).unwrap();
    crate::paths::private_file_lock::PrivateFileLock::try_acquire_existing_in(
        directory,
        &staged.artifacts.retirement_writer_lock_path(),
        super::constants::LOCK_RECORD_MAX_BYTES,
    )
    .unwrap()
    .unwrap()
}

pub(super) fn write_private(path: &Path, bytes: &[u8]) {
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(crate::paths::constants::PRIVATE_FILE_MODE)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}

pub(super) fn entry_names(path: &Path) -> Vec<std::ffi::OsString> {
    let mut names: Vec<_> = std::fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    names.sort();
    names
}

pub(super) fn artifacts(fixture: &Fixture, invocation: u128) -> CodexHookArtifacts {
    CodexHookArtifacts::new(
        &fixture.paths,
        fixture.identity.terminal_id,
        fixture.identity.runtime_epoch,
        Uuid::from_u128(invocation),
    )
    .unwrap()
}

fn create_writer_lock(
    directory: &crate::paths::directory::PrivateDirectory,
    artifacts: &CodexHookArtifacts,
) {
    let entry = crate::paths::private_file::PrivateFileEntry::from_directory(
        directory.try_clone().unwrap(),
        &artifacts.retirement_writer_lock_path(),
    )
    .unwrap();
    drop(
        entry
            .open_rw_or_create(super::constants::LOCK_RECORD_MAX_BYTES)
            .unwrap(),
    );
}
