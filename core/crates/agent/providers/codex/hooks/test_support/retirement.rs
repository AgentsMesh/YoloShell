use super::admission_test_support::Fixture as AdmissionFixture;
use crate::agent::hooks::HookEnvelope;
use crate::codex_hooks::{
    CodexAdmissionAuthority, CodexHookIntake, CodexRetirementSeal, CodexRetirementWriterFence,
};
use crate::process::{ExactProcessGone, ProcessIdentityObservation};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;

pub(super) const MALFORMED_JSON: &[u8] = b"{";
const REPLACEMENT_FILE_NAME: &str = "retirement-test-replacement.json";
const PRIVATE_FILE_MODE: u32 = crate::paths::constants::PRIVATE_FILE_MODE;

pub(super) struct RetirementFixture {
    pub(super) admission: AdmissionFixture,
    pub(super) authority: CodexAdmissionAuthority,
}

#[derive(Clone, Copy)]
pub(super) enum SealMutation {
    Deleted,
    DifferentContent,
    SameContentNewIdentity,
}

impl RetirementFixture {
    pub(super) fn new() -> Self {
        Self::from_admission(AdmissionFixture::new())
    }

    pub(super) fn with_identity(identity: crate::agent::hooks::HookIdentity) -> Self {
        Self::from_admission(AdmissionFixture::with_identity(identity))
    }

    fn from_admission(admission: AdmissionFixture) -> Self {
        let intake = CodexHookIntake::for_identity(
            admission.artifacts.intake_directory(),
            &admission.identity,
        )
        .unwrap();
        // Lock-creation durability is covered separately; retirement tests begin after provisioning.
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(PRIVATE_FILE_MODE)
            .open(admission.artifacts.retirement_writer_lock_path())
            .unwrap();
        drop(intake);
        let storage = admission.storage();
        let publication = storage.publish(&admission.record()).unwrap();
        let authority = CodexAdmissionAuthority::from_publication(storage, publication).unwrap();
        Self {
            admission,
            authority,
        }
    }

    pub(super) fn admission(&self) -> &CodexAdmissionAuthority {
        &self.authority
    }

    pub(super) fn home(&self) -> &Path {
        self.admission.paths.home()
    }

    pub(super) fn owner(&self) -> &crate::agent::hooks::HookIdentity {
        &self.admission.identity
    }

    pub(super) fn recovered_admission(&self) -> CodexAdmissionAuthority {
        CodexAdmissionAuthority::recover(self.admission.storage())
            .unwrap()
            .unwrap()
    }

    pub(super) fn admission_path(&self) -> &Path {
        self.authority.path()
    }

    pub(super) fn intake(&self) -> CodexHookIntake {
        CodexHookIntake::for_identity(
            self.admission.artifacts.intake_directory(),
            &self.admission.identity,
        )
        .unwrap()
    }

    pub(super) fn envelope(&self) -> HookEnvelope {
        super::intake_test_support::envelope(&self.admission.identity)
    }

    pub(super) fn writer(&self) -> CodexRetirementWriterFence {
        self.intake().acquire_writer_fence().unwrap()
    }

    pub(super) fn seal(&self) -> CodexRetirementSeal {
        let process = crate::process::ProcessIdentity {
            pid: self.admission.identity.process.pid,
            start_time: self.admission.identity.process.start_time,
        };
        let gone =
            ExactProcessGone::capture_for_test(process, ProcessIdentityObservation::Gone).unwrap();
        CodexRetirementSeal::capture(&self.authority, gone).unwrap()
    }

    pub(super) fn seal_path(&self) -> std::path::PathBuf {
        self.admission.artifacts.retirement_seal_path()
    }

    pub(super) fn retirement_path(&self) -> std::path::PathBuf {
        self.admission.artifacts.retirement_path()
    }

    pub(super) fn snapshot_path(&self) -> std::path::PathBuf {
        self.admission.artifacts.snapshot_path()
    }

    pub(super) fn invocation_directory(&self) -> &Path {
        self.admission.artifacts.directory()
    }
}

pub(super) fn replace_same_content(path: &Path) {
    replace_with(path, &std::fs::read(path).unwrap());
}

pub(super) fn replace_with(path: &Path, contents: &[u8]) {
    let replacement = path.with_file_name(REPLACEMENT_FILE_NAME);
    std::fs::write(&replacement, contents).unwrap();
    std::fs::set_permissions(
        &replacement,
        std::fs::Permissions::from_mode(PRIVATE_FILE_MODE),
    )
    .unwrap();
    std::fs::rename(replacement, path).unwrap();
}

pub(super) fn mutate_seal(fixture: &RetirementFixture, mutation: SealMutation) {
    let path = fixture.seal_path();
    match mutation {
        SealMutation::Deleted => std::fs::remove_file(path).unwrap(),
        SealMutation::DifferentContent => {
            let foreign = RetirementFixture::new();
            replace_with(&path, &serde_json::to_vec_pretty(&foreign.seal()).unwrap());
        }
        SealMutation::SameContentNewIdentity => replace_same_content(&path),
    }
}

pub(super) fn replace_snapshot_after_retirement_install(snapshot: &Path, retirement: &Path) {
    arm_snapshot_replacement(snapshot.to_path_buf(), retirement.to_path_buf());
}

fn arm_snapshot_replacement(snapshot: std::path::PathBuf, retirement: std::path::PathBuf) {
    super::super::snapshot_cursor::set_before_publication_hook(move || {
        if retirement.exists() {
            crate::codex_hooks::replace_snapshot_same_content(&snapshot);
        } else {
            arm_snapshot_replacement(snapshot, retirement);
        }
    });
}
