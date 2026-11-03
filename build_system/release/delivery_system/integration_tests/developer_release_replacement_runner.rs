use super::developer_release_fixture::FakeDeveloperRunner;
use crate::developer_release_command::DeveloperReleaseCommandRunner;
use crate::developer_release_constants::NOTARY_SUBMISSION_NAME;
use anyhow::{Result, anyhow};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_verification::test_support::unseal;

const REPLACED_RECEIPT_NAME: &str = "retained-acceptance-receipt.json";
const REPLACED_CANDIDATE_NAME: &str = "retained-candidate";
const REPLACEMENT_LOCK_ERROR: &str = "replacement runner lock is poisoned";

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ReplacementMoment {
    BeforeSign,
    BeforeNotarize,
    DuringRecoveryVerification,
}

enum Replacement {
    Receipt(PathBuf),
    Candidate(PathBuf),
}

pub struct ReplacingDeveloperRunner {
    inner: FakeDeveloperRunner,
    moment: ReplacementMoment,
    replacement: Mutex<Option<Replacement>>,
    signings: AtomicUsize,
}

impl ReplacingDeveloperRunner {
    pub fn receipt(path: PathBuf, moment: ReplacementMoment) -> Self {
        Self::new(Replacement::Receipt(path), moment)
    }

    pub fn candidate_before_sign(path: PathBuf) -> Self {
        Self::new(Replacement::Candidate(path), ReplacementMoment::BeforeSign)
    }

    pub fn signings(&self) -> usize {
        self.signings.load(Ordering::SeqCst)
    }

    pub fn notarizations(&self) -> usize {
        self.inner.notarizations.load(Ordering::SeqCst)
    }

    fn new(replacement: Replacement, moment: ReplacementMoment) -> Self {
        Self {
            inner: FakeDeveloperRunner::passing(),
            moment,
            replacement: Mutex::new(Some(replacement)),
            signings: AtomicUsize::new(0),
        }
    }

    fn replace_once(&self, moment: ReplacementMoment) -> Result<()> {
        if self.moment != moment {
            return Ok(());
        }
        let replacement = self
            .replacement
            .lock()
            .map_err(|_| anyhow!(REPLACEMENT_LOCK_ERROR))?
            .take();
        match replacement {
            Some(Replacement::Receipt(path)) => replace_receipt(path),
            Some(Replacement::Candidate(path)) => replace_candidate(path),
            None => Ok(()),
        }
    }
}

impl DeveloperReleaseCommandRunner for ReplacingDeveloperRunner {
    fn extract(&self, workspace: &DirectoryHandle, archive: Arc<std::fs::File>) -> Result<()> {
        self.inner.extract(workspace, archive)
    }

    fn sign(&self, workspace: &DirectoryHandle) -> Result<()> {
        self.signings.fetch_add(1, Ordering::SeqCst);
        self.inner.sign(workspace)
    }

    fn remove_signature(&self, workspace: &DirectoryHandle, path: &str) -> Result<()> {
        self.replace_once(ReplacementMoment::BeforeSign)?;
        self.inner.remove_signature(workspace, path)
    }

    fn verify_signature(&self, workspace: &DirectoryHandle, extracted: bool) -> Result<()> {
        self.replace_once(ReplacementMoment::DuringRecoveryVerification)?;
        self.inner.verify_signature(workspace, extracted)
    }

    fn create_archive(
        &self,
        workspace: &DirectoryHandle,
        source: &str,
        destination: &str,
    ) -> Result<()> {
        self.inner.create_archive(workspace, source, destination)?;
        if destination == NOTARY_SUBMISSION_NAME {
            self.replace_once(ReplacementMoment::BeforeNotarize)?;
        }
        Ok(())
    }

    fn notarize(
        &self,
        workspace: &DirectoryHandle,
        profile: &str,
        archive: Arc<std::fs::File>,
    ) -> Result<String> {
        self.inner.notarize(workspace, profile, archive)
    }

    fn staple(&self, workspace: &DirectoryHandle, extracted: bool) -> Result<()> {
        self.inner.staple(workspace, extracted)
    }

    fn validate_ticket(&self, workspace: &DirectoryHandle, extracted: bool) -> Result<()> {
        self.inner.validate_ticket(workspace, extracted)
    }

    fn assess(&self, workspace: &DirectoryHandle, extracted: bool) -> Result<()> {
        self.inner.assess(workspace, extracted)
    }
}

fn replace_receipt(path: PathBuf) -> Result<()> {
    let bytes = fs::read(&path)?;
    let retained = path.with_file_name(REPLACED_RECEIPT_NAME);
    fs::rename(&path, retained)?;
    fs::write(path, bytes).map_err(Into::into)
}

fn replace_candidate(path: PathBuf) -> Result<()> {
    unseal(&path)?;
    let retained = path.with_file_name(REPLACED_CANDIDATE_NAME);
    fs::rename(&path, retained)?;
    fs::create_dir(path).map_err(Into::into)
}
