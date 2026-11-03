use super::developer_release_fixture_archive::{
    DistributedArchiveKind, MUTATED_EXECUTABLE_BYTES, RESOURCE_BYTES, candidate_archive,
    distributed_archive, install_executables, mutated_signed_archive, set_mode,
};
use crate::developer_release_command::DeveloperReleaseCommandRunner;
use crate::developer_release_constants::{
    NOTARY_SUBMISSION_NAME, NOTARY_TICKET_ARCHIVE_PATH, SIGNED_ARCHIVE_PATHS,
};
use anyhow::{Result, bail};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use yolo_release_archive_audit::constants::{
    DIRECTORY_MODE, EXECUTABLE_MODE, REGULAR_RESOURCE_MODE,
};
use yolo_release_contracts::{APP_NAME, EXTRACTION_DIRECTORY_NAME};
use yolo_release_filesystem::DirectoryHandle;

pub const TEST_NOTARIZATION_ID: &str = "00000000-0000-0000-0000-000000000001";
pub const INJECTED_NOTARY_FAILURE: &str = "injected notary failure";
pub const INJECTED_ASSESSMENT_FAILURE: &str = "injected assessment failure";
pub const POSTPUBLICATION_ASSESSMENT_CALL: usize = 3;
const TICKET_BYTES: &[u8] = b"notarization-ticket";
const REBOUND_DESTINATION_NAME: &str = "rebound-original";

pub struct FakeDeveloperRunner {
    notary_fails: bool,
    assessment_failure: Option<usize>,
    archive_kind: DistributedArchiveKind,
    mutate_signing: bool,
    mutate_submission: bool,
    rebind: Option<(usize, PathBuf)>,
    pub notarizations: AtomicUsize,
    pub signature_verifications: AtomicUsize,
    pub ticket_validations: AtomicUsize,
    pub assessments: AtomicUsize,
}

impl FakeDeveloperRunner {
    pub fn passing() -> Self {
        Self::new()
    }

    pub fn failing_notary() -> Self {
        let mut runner = Self::new();
        runner.notary_fails = true;
        runner
    }

    pub fn failing_assessment(call: usize) -> Self {
        let mut runner = Self::new();
        runner.assessment_failure = Some(call);
        runner
    }

    pub fn divergent_archive(kind: DistributedArchiveKind) -> Self {
        let mut runner = Self::new();
        runner.archive_kind = kind;
        runner
    }

    pub fn mutating_signing() -> Self {
        let mut runner = Self::new();
        runner.mutate_signing = true;
        runner
    }

    pub fn mutating_submission() -> Self {
        let mut runner = Self::new();
        runner.mutate_submission = true;
        runner
    }

    pub fn rebinding(call: usize, destination: PathBuf) -> Self {
        let mut runner = Self::new();
        runner.rebind = Some((call, destination));
        runner
    }

    fn new() -> Self {
        Self {
            notary_fails: false,
            assessment_failure: None,
            archive_kind: DistributedArchiveKind::Exact,
            mutate_signing: false,
            mutate_submission: false,
            rebind: None,
            notarizations: AtomicUsize::new(0),
            signature_verifications: AtomicUsize::new(0),
            ticket_validations: AtomicUsize::new(0),
            assessments: AtomicUsize::new(0),
        }
    }
}

impl DeveloperReleaseCommandRunner for FakeDeveloperRunner {
    fn extract(&self, workspace: &DirectoryHandle, _archive: Arc<std::fs::File>) -> Result<()> {
        let contents = workspace
            .display_path()
            .join(EXTRACTION_DIRECTORY_NAME)
            .join(APP_NAME)
            .join("Contents");
        fs::create_dir_all(&contents)?;
        fs::write(contents.join("payload"), RESOURCE_BYTES)?;
        install_executables(&contents)?;
        set_mode(contents.parent().unwrap(), u32::from(DIRECTORY_MODE))?;
        set_mode(&contents, u32::from(DIRECTORY_MODE))?;
        set_mode(&contents.join("payload"), u32::from(REGULAR_RESOURCE_MODE))
    }

    fn sign(&self, workspace: &DirectoryHandle) -> Result<()> {
        if self.mutate_signing {
            let executable = workspace
                .display_path()
                .join(EXTRACTION_DIRECTORY_NAME)
                .join(SIGNED_ARCHIVE_PATHS[0]);
            fs::write(&executable, MUTATED_EXECUTABLE_BYTES)?;
            set_mode(&executable, u32::from(EXECUTABLE_MODE))?;
        }
        Ok(())
    }

    fn remove_signature(&self, _workspace: &DirectoryHandle, _path: &str) -> Result<()> {
        Ok(())
    }

    fn verify_signature(&self, _workspace: &DirectoryHandle, _extracted: bool) -> Result<()> {
        self.signature_verifications.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn create_archive(
        &self,
        workspace: &DirectoryHandle,
        _source: &str,
        destination: &str,
    ) -> Result<()> {
        let bytes = if destination == NOTARY_SUBMISSION_NAME {
            if self.mutate_submission {
                mutated_signed_archive()
            } else {
                candidate_archive()
            }
        } else {
            distributed_archive(self.archive_kind)
        };
        fs::write(workspace.display_path().join(destination), bytes)?;
        Ok(())
    }

    fn notarize(
        &self,
        _workspace: &DirectoryHandle,
        _profile: &str,
        _archive: Arc<std::fs::File>,
    ) -> Result<String> {
        self.notarizations.fetch_add(1, Ordering::SeqCst);
        if self.notary_fails {
            bail!(INJECTED_NOTARY_FAILURE);
        }
        Ok(TEST_NOTARIZATION_ID.to_owned())
    }

    fn staple(&self, workspace: &DirectoryHandle, _extracted: bool) -> Result<()> {
        let ticket = workspace
            .display_path()
            .join(EXTRACTION_DIRECTORY_NAME)
            .join(NOTARY_TICKET_ARCHIVE_PATH);
        fs::write(&ticket, TICKET_BYTES)?;
        set_mode(&ticket, u32::from(REGULAR_RESOURCE_MODE))
    }

    fn validate_ticket(&self, _workspace: &DirectoryHandle, _extracted: bool) -> Result<()> {
        self.ticket_validations.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn assess(&self, _workspace: &DirectoryHandle, _extracted: bool) -> Result<()> {
        let call = self.assessments.fetch_add(1, Ordering::SeqCst) + 1;
        if self.assessment_failure == Some(call) {
            bail!(INJECTED_ASSESSMENT_FAILURE);
        }
        if let Some((expected, destination)) = &self.rebind
            && *expected == call
        {
            let moved = destination.with_file_name(REBOUND_DESTINATION_NAME);
            fs::rename(destination, moved)?;
            fs::create_dir(destination)?;
        }
        Ok(())
    }
}
