use super::runtime_intake_test_support::RoutedFixture;
use super::support::FIRST_EVENT_SEQUENCE;
use crate::agent::hooks::{EventSequence, HookEnvelope, HookIdentity};
use crate::codex_hooks::{
    CodexAdmissionAuthority, CodexAdmissionStorage, CodexHookArtifacts, CodexHookIntake,
    CodexIntakeOccurrence, CodexRetirementSeal, CodexRetirementSealArtifact,
    CodexRetirementSealPublication, CodexRetirementWriterFence,
};
use crate::process::{ExactProcessGone, ProcessIdentityObservation};
use crate::worker::codex_hook::router::{
    CodexRetirementPermit, CodexRetirementReadiness, CodexSessionRouter,
};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use uuid::Uuid;

const FOREIGN_HOME_DIRECTORY: &str = "retirement-foreign-home";
const REPLACEMENT_FILE_NAME: &str = "retirement-occurrence-replacement";
const PRIVATE_FILE_MODE: u32 = crate::paths::constants::PRIVATE_FILE_MODE;

pub(super) struct RetirementResources {
    permit: CodexRetirementPermit,
    writer: CodexRetirementWriterFence,
    seal: CodexRetirementSealPublication,
}

pub(super) struct ForeignLineage {
    _directory: tempfile::TempDir,
    writer: CodexRetirementWriterFence,
    seal: CodexRetirementSealPublication,
}

impl RetirementResources {
    pub(super) fn new(fixture: &RoutedFixture) -> Self {
        Self::for_route(fixture.router(), &fixture.identity)
    }

    pub(super) fn for_route(router: &CodexSessionRouter, identity: &HookIdentity) -> Self {
        let entry = router.resolve(identity).unwrap();
        let permit = ready(router.begin_retirement(&entry).unwrap());
        let admission = permit.entry().admission().authority();
        let writer = admission.acquire_retirement_writer_fence().unwrap();
        let seal = writer
            .publish_seal(&capture_seal(admission), admission)
            .unwrap();
        Self {
            permit,
            writer,
            seal,
        }
    }

    pub(super) fn permit(&self) -> &CodexRetirementPermit {
        &self.permit
    }

    pub(super) fn writer(&self) -> &CodexRetirementWriterFence {
        &self.writer
    }

    pub(super) fn seal(&self) -> &CodexRetirementSealArtifact {
        self.seal.artifact()
    }

    pub(super) fn admission(&self) -> &CodexAdmissionAuthority {
        self.permit.entry().admission().authority()
    }
}

impl ForeignLineage {
    pub(super) fn new(fixture: &RoutedFixture) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let paths = crate::paths::Paths::new(directory.path().join(FOREIGN_HOME_DIRECTORY));
        paths
            .ensure_session(fixture.identity.terminal_id)
            .unwrap();
        let artifacts = CodexHookArtifacts::new(
            &paths,
            fixture.identity.terminal_id,
            fixture.identity.runtime_epoch,
            fixture.identity.invocation_id,
        )
        .unwrap();
        let storage = CodexAdmissionStorage::for_identity(&artifacts, &fixture.identity).unwrap();
        let publication = storage.publish_for_test(&fixture.record).unwrap();
        let admission = CodexAdmissionAuthority::from_publication(storage, publication).unwrap();
        let writer = admission.acquire_retirement_writer_fence().unwrap();
        let seal = writer
            .publish_seal(&capture_seal(&admission), &admission)
            .unwrap();
        Self {
            _directory: directory,
            writer,
            seal,
        }
    }

    pub(super) fn writer(&self) -> &CodexRetirementWriterFence {
        &self.writer
    }

    pub(super) fn seal(&self) -> &CodexRetirementSealArtifact {
        self.seal.artifact()
    }
}

pub(super) fn persisted_occurrence(
    fixture: &RoutedFixture,
) -> (CodexHookIntake, CodexIntakeOccurrence) {
    let intake = intake(fixture);
    intake
        .persist_for_test(unassigned(fixture.start(FIRST_EVENT_SEQUENCE)))
        .unwrap();
    let occurrence = intake.scan().unwrap().occurrences.pop().unwrap();
    (intake, occurrence)
}

pub(super) fn intake(fixture: &RoutedFixture) -> CodexHookIntake {
    CodexHookIntake::for_identity(intake_directory(fixture), &fixture.identity).unwrap()
}

pub(super) fn intake_directory(fixture: &RoutedFixture) -> std::path::PathBuf {
    let artifacts = CodexHookArtifacts::new(
        &fixture.paths,
        fixture.identity.terminal_id,
        fixture.identity.runtime_epoch,
        fixture.identity.invocation_id,
    )
    .unwrap();
    artifacts.intake_directory()
}

pub(super) fn assigned(occurrence: &CodexIntakeOccurrence) -> HookEnvelope {
    let mut event = occurrence.envelope().clone();
    event.sequence = EventSequence::FIRST;
    event
}

pub(super) fn capture_seal(admission: &CodexAdmissionAuthority) -> CodexRetirementSeal {
    let owner = &admission.record().request().identity;
    let process = crate::process::ProcessIdentity {
        pid: owner.process.pid,
        start_time: owner.process.start_time,
    };
    let gone =
        ExactProcessGone::capture_for_test(process, ProcessIdentityObservation::Gone).unwrap();
    CodexRetirementSeal::capture(admission, gone).unwrap()
}

pub(super) fn replace_occurrence(directory: &Path) {
    let mut entries = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 1);
    let original = entries.pop().unwrap();
    replace_same_contents(&original, REPLACEMENT_FILE_NAME);
}

pub(super) fn replace_same_contents(path: &Path, replacement_name: &str) {
    let replacement = path.parent().unwrap().join(replacement_name);
    std::fs::write(&replacement, std::fs::read(path).unwrap()).unwrap();
    std::fs::set_permissions(
        &replacement,
        std::fs::Permissions::from_mode(PRIVATE_FILE_MODE),
    )
    .unwrap();
    std::fs::rename(replacement, path).unwrap();
}

fn unassigned(mut event: HookEnvelope) -> HookEnvelope {
    event.event_id = Uuid::nil();
    event.sequence = EventSequence::ZERO;
    event
}

fn ready(readiness: CodexRetirementReadiness) -> CodexRetirementPermit {
    match readiness {
        CodexRetirementReadiness::Ready(permit) => permit,
        CodexRetirementReadiness::Pending | CodexRetirementReadiness::Missing => unreachable!(),
    }
}
