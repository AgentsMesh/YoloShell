use super::process_support::stub_process_source;
use super::runtime_intake_test_support::{directory, executable};
use super::support::{envelope, prepare_writer_lock};
use crate::agent::hooks::{EventKind, Generation, HookIdentity, ProcessIdentity};
use crate::codex_hooks::{CodexAdmissionContext, CodexAdmissionRecord, CodexAdmissionStorage};
use crate::codex_hooks::{CodexHookArtifacts, CodexHookIntake, CodexOutbox};
use crate::worker::codex_hook::authority_registry::CodexAuthorityRegistry;
use crate::worker::codex_hook::router::{
    CodexRouteEntry, CodexRouteRetirement, CodexSessionRouter, DurableCodexAdmission,
};
use crate::worker::codex_hook::runtime::CodexHookRuntime;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;
pub(super) const FIRST_INVOCATION: u64 = 1;
pub(super) const SECOND_INVOCATION: u64 = 2;
pub(super) const EXPECTED_ROUTE_COUNT: usize = 2;
pub(super) const EXPECTED_SINGLE_COUNT: usize = 1;
const HOME_DIRECTORY: &str = "intake-task-home";
const EXECUTABLE_DIRECTORY: &str = "intake-task-executables";
const CWD_DIRECTORY: &str = "intake-task-project";
const DATA_HOME_DIRECTORY: &str = "intake-task-data-home";
const HOOK_FILE_NAME: &str = "codex-hook";
const REAL_FILE_NAME: &str = "codex-real";
const BROKEN_INTAKE_CONTENT: &[u8] = b"not-a-directory";
pub(super) struct IntakeTaskFixture {
    _root: tempfile::TempDir,
    paths: crate::paths::Paths,
    registry: Arc<CodexAuthorityRegistry>,
    router: Arc<CodexSessionRouter>,
    shell: crate::process::ProcessIdentity,
    process: crate::process::ProcessIdentity,
    cwd: PathBuf,
    data_home: PathBuf,
    hook: PathBuf,
    real: PathBuf,
}
pub(super) struct IntakeRoute {
    pub(super) identity: HookIdentity,
    pub(super) runtime: Arc<CodexHookRuntime>,
    pub(super) entry: Arc<CodexRouteEntry>,
    intake: CodexHookIntake,
    intake_path: PathBuf,
}
impl IntakeTaskFixture {
    pub(super) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let canonical = root.path().canonicalize().unwrap();
        let paths = crate::paths::Paths::new(canonical.join(HOME_DIRECTORY));
        let terminal_id = Uuid::new_v4();
        let runtime_epoch = Uuid::new_v4();
        paths.ensure_session(terminal_id).unwrap();
        let executables = directory(&canonical.join(EXECUTABLE_DIRECTORY));
        let process = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
        let router =
            Arc::new(CodexSessionRouter::new(paths.clone(), terminal_id, runtime_epoch).unwrap());
        Self {
            _root: root,
            paths,
            registry: Arc::new(CodexAuthorityRegistry::default()),
            router,
            shell: process,
            process,
            cwd: directory(&canonical.join(CWD_DIRECTORY)),
            data_home: directory(&canonical.join(DATA_HOME_DIRECTORY)),
            hook: executable(&executables.join(HOOK_FILE_NAME)),
            real: executable(&executables.join(REAL_FILE_NAME)),
        }
    }
    pub(super) fn router(&self) -> Arc<CodexSessionRouter> {
        Arc::clone(&self.router)
    }
    pub(super) fn route(&self, invocation: u64) -> IntakeRoute {
        let identity = HookIdentity {
            terminal_id: self.router.terminal_id(),
            runtime_epoch: self.router.runtime_epoch(),
            invocation_id: Uuid::from_u128(u128::from(invocation)),
            hook_token: Uuid::new_v4(),
            process: ProcessIdentity::new(self.process.pid, self.process.start_time),
            generation: Generation::INITIAL,
        };
        let request = CodexAdmissionContext::new(
            identity.clone(),
            self.cwd.clone(),
            self.data_home.clone(),
            self.hook.clone(),
            self.real.clone(),
        );
        let record =
            CodexAdmissionRecord::capture_with_route_revision(&request, invocation).unwrap();
        let artifacts = CodexHookArtifacts::new(
            &self.paths,
            self.router.terminal_id(),
            self.router.runtime_epoch(),
            identity.invocation_id,
        )
        .unwrap();
        let runtime = CodexHookRuntime::new_test_in_registry(
            &self.paths,
            Arc::clone(&self.registry),
            self.router.terminal_id(),
            self.router.runtime_epoch(),
            self.shell,
            identity.clone(),
            crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
            stub_process_source(self.process),
        )
        .unwrap()
        .0;
        let storage = CodexAdmissionStorage::for_identity(&artifacts, &identity).unwrap();
        prepare_writer_lock(&self.paths, &identity);
        let publication = storage.publish_for_test(&record).unwrap();
        let admission = DurableCodexAdmission::from_publication(storage, publication).unwrap();
        let lease = self
            .router
            .install_or_replay(admission, |_| Ok(Arc::clone(&runtime)))
            .unwrap();
        let entry = Arc::clone(lease.entry());
        lease.settle().unwrap();
        let intake_path = artifacts.intake_directory();
        let intake = CodexHookIntake::for_identity(&intake_path, &identity).unwrap();
        IntakeRoute {
            identity,
            runtime,
            entry,
            intake,
            intake_path,
        }
    }
    pub(super) fn retire(&self, route: &IntakeRoute) -> CodexRouteRetirement {
        self.router.retire(&route.entry).unwrap()
    }
}

impl IntakeRoute {
    pub(super) fn persist_start(&self) {
        self.commit(start_event());
    }

    pub(super) fn persist_raw_start(&self) {
        self.persist(start_event());
    }

    pub(super) fn persist_raw_prompt(&self) {
        self.persist(EventKind::UserPromptSubmit);
    }

    pub(super) fn pending(&self) -> usize {
        self.intake.scan().unwrap().occurrences.len()
    }

    pub(super) fn intake_path(&self) -> &Path {
        &self.intake_path
    }

    pub(super) fn break_intake_storage(&self) {
        std::fs::remove_dir(&self.intake_path).unwrap();
        std::fs::write(&self.intake_path, BROKEN_INTAKE_CONTENT).unwrap();
    }

    pub(super) fn remove_admission(&self) {
        std::fs::remove_file(self.entry.admission().path()).unwrap();
    }

    fn commit(&self, event: EventKind) {
        let occurrence = self.persist(event);
        let identity = self
            .intake
            .find_exact_occurrence(&occurrence)
            .unwrap()
            .unwrap()
            .durable_identity()
            .unwrap();
        CodexOutbox::for_identity(self.runtime.outbox_path(), &self.identity)
            .unwrap()
            .enqueue_admitted_for_test(occurrence, Some(&identity), |_, _| Ok(()), |_, _, _| Ok(()))
            .unwrap();
    }

    fn persist(&self, event: EventKind) -> crate::agent::hooks::HookEnvelope {
        let event = envelope(
            self.identity.clone(),
            crate::agent::hooks::ZERO_SEQUENCE,
            Uuid::nil(),
            event,
        );
        self.intake.persist_occurrence_for_test(event).unwrap()
    }
}

fn start_event() -> EventKind {
    EventKind::SessionStart {
        source: crate::agent::hooks::SessionStartSource::Startup,
    }
}
