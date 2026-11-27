use super::process_support::stub_process_source;
use super::recovery_fixture_files::{identity, request};
use super::support::{Fixture, envelope};
use crate::agent::hooks::{EventKind, HookIdentity};
use crate::codex_hooks::{
    CodexAdmissionAuthority, CodexAdmissionRecord, CodexAdmissionStorage, CodexHookArtifacts,
    CodexHookIntake, CodexOutbox, CodexRetirementState,
};
use crate::process::ProcessIdentityObservation;
use crate::worker::codex_hook::authority_registry::CodexAuthorityRegistry;
use crate::worker::codex_hook::recovery::CodexRecoveryContext;
use crate::worker::codex_hook::router::CodexSessionRouter;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

const HOME_DIRECTORY: &str = "recovery-home";
const INITIAL_ROUTE_REVISION: u64 = 1;
const SUCCESSOR_ROUTE_REVISION: u64 = 2;

pub(super) struct RecoveryFixture {
    _root: tempfile::TempDir,
    pub(super) paths: crate::paths::Paths,
    pub(super) identity: HookIdentity,
    pub(super) admission: CodexAdmissionAuthority,
    pub(super) shell: crate::process::ProcessIdentity,
    process: crate::process::ProcessIdentity,
    artifacts: CodexHookArtifacts,
}

impl RecoveryFixture {
    pub(super) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let canonical = root.path().canonicalize().unwrap();
        let paths = crate::paths::Paths::new(canonical.join(HOME_DIRECTORY));
        paths.ensure_session(Fixture::terminal_id()).unwrap();
        let process = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
        let identity = identity(process);
        let request = request(&canonical, identity.clone());
        let record =
            CodexAdmissionRecord::capture_with_route_revision(&request, INITIAL_ROUTE_REVISION)
                .unwrap();
        let artifacts = CodexHookArtifacts::new(
            &paths,
            Fixture::terminal_id(),
            Fixture::runtime_epoch(),
            identity.invocation_id,
        )
        .unwrap();
        let storage = CodexAdmissionStorage::for_identity(&artifacts, &identity).unwrap();
        let publication = storage.publish_for_test(&record).unwrap();
        let admission = CodexAdmissionAuthority::from_publication(storage, publication).unwrap();
        CodexHookIntake::for_identity(artifacts.intake_directory(), &identity).unwrap();
        Self {
            _root: root,
            paths,
            identity,
            admission,
            shell: process,
            process,
            artifacts,
        }
    }

    pub(super) fn try_recover(
        &self,
        observation: ProcessIdentityObservation,
    ) -> (CodexSessionRouter, anyhow::Result<()>) {
        let router = CodexSessionRouter::new(
            self.paths.clone(),
            Fixture::terminal_id(),
            Fixture::runtime_epoch(),
        )
        .unwrap();
        let context = CodexRecoveryContext::new(
            &self.paths,
            Arc::new(CodexAuthorityRegistry::default()),
            &router,
            self.shell,
            stub_process_source(self.process),
        );
        let result = context
            .recover_observed(observation)
            .map(|round| round.accept());
        (router, result)
    }

    pub(super) fn retirement_state(&self) -> CodexRetirementState {
        self.admission
            .acquire_retirement_writer_fence()
            .unwrap()
            .read_state(&self.admission)
            .unwrap()
    }

    pub(super) fn publish_seal(&self) {
        let writer = self.admission.acquire_retirement_writer_fence().unwrap();
        let seal = super::runtime_retirement_intake_test_support::capture_seal(&self.admission);
        writer.publish_seal(&seal, &self.admission).unwrap();
    }

    pub(super) fn persist_start(&self) {
        let occurrence = self.persist_raw_start();
        let identity = self
            .intake()
            .find_exact_occurrence(&occurrence)
            .unwrap()
            .unwrap()
            .durable_identity()
            .unwrap();
        CodexOutbox::for_identity(self.artifacts.outbox_path(), &self.identity)
            .unwrap()
            .enqueue_admitted_for_test(occurrence, Some(&identity), |_, _| Ok(()), |_, _, _| Ok(()))
            .unwrap();
    }

    pub(super) fn persist_raw_start(&self) -> crate::agent::hooks::HookEnvelope {
        self.intake()
            .persist_occurrence_for_test(envelope(
                self.identity.clone(),
                crate::agent::hooks::ZERO_SEQUENCE,
                Uuid::nil(),
                EventKind::SessionStart {
                    source: crate::agent::hooks::SessionStartSource::Startup,
                },
            ))
            .unwrap()
    }

    pub(super) fn additional_admission(&self) -> CodexAdmissionAuthority {
        self.additional_admission_with_invocation(Uuid::new_v4())
    }

    pub(super) fn additional_admission_with_invocation(
        &self,
        invocation_id: Uuid,
    ) -> CodexAdmissionAuthority {
        let mut identity = identity(self.process);
        identity.invocation_id = invocation_id;
        let mut request = self.admission.record().request().clone();
        request.identity = identity.clone();
        let record =
            CodexAdmissionRecord::capture_with_route_revision(&request, SUCCESSOR_ROUTE_REVISION)
                .unwrap();
        let artifacts = CodexHookArtifacts::new(
            &self.paths,
            Fixture::terminal_id(),
            Fixture::runtime_epoch(),
            identity.invocation_id,
        )
        .unwrap();
        let storage = CodexAdmissionStorage::for_identity(&artifacts, &identity).unwrap();
        let publication = storage.publish_for_test(&record).unwrap();
        CodexHookIntake::for_identity(artifacts.intake_directory(), &identity).unwrap();
        CodexAdmissionAuthority::from_publication(storage, publication).unwrap()
    }

    pub(super) fn intake(&self) -> CodexHookIntake {
        CodexHookIntake::for_identity(self.artifacts.intake_directory(), &self.identity).unwrap()
    }

    pub(super) fn intake_directory(&self) -> PathBuf {
        self.artifacts.intake_directory()
    }

    pub(super) fn outbox_path(&self) -> PathBuf {
        self.artifacts.outbox_path()
    }

    pub(super) fn seal_path(&self) -> PathBuf {
        self.artifacts.retirement_seal_path_for_test()
    }

    pub(super) fn snapshot_path(&self) -> PathBuf {
        self.artifacts.snapshot_path()
    }
}
