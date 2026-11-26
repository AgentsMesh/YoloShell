use super::support::{error_text, provenance};
use crate::delivery::DeliveryRequest;
use crate::delivery_command::DeliveryCommandRunner;
use anyhow::{Result, bail};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use yolo_release_archive_audit::constants::{
    DIRECTORY_MODE, REGULAR_RESOURCE_MODE, ROOT_DIRECTORY_ENTRY,
};
use yolo_release_archive_audit_test_support::{FixtureEntry, build};
use yolo_release_candidate_artifact::PinnedArtifact;
use yolo_release_contracts::{APP_NAME, DELIVERY_TEMPORARY_PREFIX, EXTRACTION_DIRECTORY_NAME};
use yolo_release_foundation::digest::bytes_sha256;
use yolo_release_foundation::json::encode;
use yolo_release_verification::output::OutputTransaction;
use yolo_release_verification::test_support::unseal;
use yolo_release_verification::validation::checksum_contents;

pub const CONTENTS_DIRECTORY_ENTRY: &[u8] = b"Yolo.app/Contents/";
pub const PAYLOAD_PATH: &str = "Yolo.app/Contents/payload";
pub const PAYLOAD_ENTRY: &[u8] = PAYLOAD_PATH.as_bytes();
pub const EXTRA_ROOT_NAME: &str = "unexpected-root";
pub const INJECTED_SIGNATURE_FAILURE: &str = "injected signature verification failure";
pub const MALFORMED_ARCHIVE_BYTES: &[u8] = b"not-a-zip";
pub const DRIFTED_PAYLOAD_MODE: u32 = 0o600;
pub const INITIAL_EXTRACTION_CALLS: usize = 1;
pub const PREPUBLICATION_SIGNATURE_CALLS: usize = 1;
pub const COMPLETE_DELIVERY_SIGNATURE_CALLS: usize = 2;

#[derive(Clone, Copy)]
pub enum ExtractionKind {
    Exact,
    ExtraRoot,
    PermissionDrift,
}

pub struct FakeRunner {
    kind: ExtractionKind,
    signature_fails: bool,
    extraction_calls: AtomicUsize,
    signature_calls: AtomicUsize,
}

impl FakeRunner {
    pub fn new(kind: ExtractionKind, signature_fails: bool) -> Self {
        Self {
            kind,
            signature_fails,
            extraction_calls: AtomicUsize::new(0),
            signature_calls: AtomicUsize::new(0),
        }
    }

    pub fn extraction_calls(&self) -> usize {
        self.extraction_calls.load(Ordering::SeqCst)
    }

    pub fn signature_calls(&self) -> usize {
        self.signature_calls.load(Ordering::SeqCst)
    }
}

impl DeliveryCommandRunner for FakeRunner {
    fn extract(&self, stage: &yolo_release_filesystem::DirectoryHandle) -> Result<()> {
        self.extraction_calls.fetch_add(1, Ordering::SeqCst);
        let extraction = stage.display_path().join(EXTRACTION_DIRECTORY_NAME);
        let app = extraction.join(APP_NAME);
        let contents = app.join("Contents");
        fs::create_dir_all(&contents)?;
        fs::write(contents.join("payload"), [])?;
        set_mode(&app, u32::from(DIRECTORY_MODE))?;
        set_mode(&contents, u32::from(DIRECTORY_MODE))?;
        let payload_mode = match self.kind {
            ExtractionKind::PermissionDrift => DRIFTED_PAYLOAD_MODE,
            _ => u32::from(REGULAR_RESOURCE_MODE),
        };
        set_mode(&contents.join("payload"), payload_mode)?;
        if matches!(self.kind, ExtractionKind::ExtraRoot) {
            fs::write(extraction.join(EXTRA_ROOT_NAME), [])?;
        }
        Ok(())
    }

    fn verify_signature(
        &self,
        _extraction: &yolo_release_filesystem::DirectoryHandle,
    ) -> Result<()> {
        self.signature_calls.fetch_add(1, Ordering::SeqCst);
        if self.signature_fails {
            bail!(INJECTED_SIGNATURE_FAILURE);
        }
        Ok(())
    }
}

pub struct DeliveryFixture {
    _root: tempfile::TempDir,
    pub candidate: PathBuf,
    pub delivery: PathBuf,
    pub moved_candidate: PathBuf,
    pub archive: Vec<u8>,
    pub expected: yolo_release_evidence_model::ReleaseProvenance,
}

impl DeliveryFixture {
    pub fn new() -> Self {
        Self::new_in(tempfile::tempdir().unwrap())
    }

    pub fn new_in(root: tempfile::TempDir) -> Self {
        Self::from_archive_in(
            build(&[
                FixtureEntry::directory(ROOT_DIRECTORY_ENTRY.as_bytes()),
                FixtureEntry::directory(CONTENTS_DIRECTORY_ENTRY),
                FixtureEntry::regular(PAYLOAD_ENTRY),
            ])
            .bytes,
            root,
        )
    }

    pub fn from_archive(archive: Vec<u8>) -> Self {
        Self::from_archive_in(archive, tempfile::tempdir().unwrap())
    }

    fn from_archive_in(archive: Vec<u8>, root: tempfile::TempDir) -> Self {
        let candidate = root.path().join("candidate");
        let delivery = root.path().join("delivery");
        let moved_candidate = root.path().join("moved-candidate");
        let artifact_root = root.path().join("artifact");
        fs::create_dir(&artifact_root).unwrap();
        let artifact = PinnedArtifact::fixture(&artifact_root, &archive).unwrap();
        let digest = bytes_sha256(&archive);
        let expected = provenance(&digest, archive.len() as u64);
        OutputTransaction::prepare(&candidate)
            .unwrap()
            .publish(
                &artifact,
                checksum_contents(&digest).as_bytes(),
                &encode(&expected).unwrap(),
            )
            .unwrap();
        Self {
            _root: root,
            candidate,
            delivery,
            moved_candidate,
            archive,
            expected,
        }
    }

    pub fn request(&self) -> DeliveryRequest<'_> {
        DeliveryRequest {
            candidate_directory: &self.candidate,
            output_directory: &self.delivery,
        }
    }

    pub fn assert_no_stage(&self) {
        let parent = self.delivery.parent().unwrap();
        assert!(fs::read_dir(parent).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(DELIVERY_TEMPORARY_PREFIX)
        }));
    }
}

impl Drop for DeliveryFixture {
    fn drop(&mut self) {
        if self.candidate.is_dir() {
            let _ = unseal(&self.candidate);
        }
    }
}

pub fn result_error<T>(result: Result<T>) -> String {
    error_text(result)
}

fn set_mode(path: &Path, mode: u32) -> Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(Into::into)
}
