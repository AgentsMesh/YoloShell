use super::super::{
    CodexRetirementPermit, CodexRetirementReadiness, CodexSessionRouter, DurableCodexAdmission,
};
use super::process_support::{StableProcessSource, directory, executable};
use crate::agent::hooks::{Generation, HookIdentity, ProcessIdentity};
use crate::codex_hooks::{
    CodexAdmissionContext, CodexAdmissionRecord, CodexAdmissionStorage, CodexHookArtifacts,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

pub(super) const CONCURRENT_INSERTS: usize = 12;
pub(super) const SINGLE_ENTRY_COUNT: usize = 1;
pub(super) const DISTINCT_ENTRY_COUNT: usize = 2;
pub(super) const TEST_ROUTER_CAPACITY: usize = 2;
const HOME_DIRECTORY: &str = "home";
const EXECUTABLE_DIRECTORY: &str = "executables";
const CWD_DIRECTORY: &str = "project";
const DATA_HOME_DIRECTORY: &str = "provider-home";
const HOOK_FILE_NAME: &str = "codex-hook";
const REAL_FILE_NAME: &str = "codex-real";
const ROUTE_REVISION_INCREMENT: u64 = 1;
const ROUTE_REVISION_EXHAUSTED: &str = "test route revision exhausted";

pub(in crate::worker::codex_hook) struct Fixture {
    pub(super) root: tempfile::TempDir,
    pub(super) paths: crate::paths::Paths,
    pub(super) terminal_id: Uuid,
    pub(super) runtime_epoch: Uuid,
    shell: crate::process::ProcessIdentity,
    process: crate::process::ProcessIdentity,
    cwd: PathBuf,
    data_home: PathBuf,
    pub(super) hook: PathBuf,
    real: PathBuf,
    route_revision: AtomicU64,
}

pub(in crate::worker::codex_hook) struct Authority {
    pub(in crate::worker::codex_hook) identity: HookIdentity,
    pub(in crate::worker::codex_hook) runtime: Arc<super::super::CodexHookRuntime>,
    pub(in crate::worker::codex_hook) request: CodexAdmissionContext,
    pub(super) record: CodexAdmissionRecord,
    pub(super) storage: CodexAdmissionStorage,
}

impl Fixture {
    pub(in crate::worker::codex_hook) fn new() -> Self {
        Self::with_scope(Uuid::new_v4(), Uuid::new_v4())
    }

    pub(super) fn with_scope(terminal_id: Uuid, runtime_epoch: Uuid) -> Self {
        let root = tempfile::tempdir().unwrap();
        let canonical = root.path().canonicalize().unwrap();
        let paths = crate::paths::Paths::new(canonical.join(HOME_DIRECTORY));
        paths.ensure_session(terminal_id).unwrap();
        let executable_directory = directory(&canonical.join(EXECUTABLE_DIRECTORY));
        let cwd = directory(&canonical.join(CWD_DIRECTORY));
        let data_home = directory(&canonical.join(DATA_HOME_DIRECTORY));
        let process = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
        Self {
            root,
            paths,
            terminal_id,
            runtime_epoch,
            shell: process,
            process,
            cwd,
            data_home,
            hook: executable(&executable_directory.join(HOOK_FILE_NAME)),
            real: executable(&executable_directory.join(REAL_FILE_NAME)),
            route_revision: AtomicU64::default(),
        }
    }

    pub(in crate::worker::codex_hook) fn router(&self) -> CodexSessionRouter {
        CodexSessionRouter::new(self.paths.clone(), self.terminal_id, self.runtime_epoch).unwrap()
    }

    pub(super) fn router_with_capacity(&self, capacity: usize) -> CodexSessionRouter {
        CodexSessionRouter::with_capacity_limit(
            self.paths.clone(),
            self.terminal_id,
            self.runtime_epoch,
            capacity,
        )
        .unwrap()
    }

    pub(in crate::worker::codex_hook) fn authority(&self) -> Authority {
        let identity = HookIdentity {
            terminal_id: self.terminal_id,
            runtime_epoch: self.runtime_epoch,
            invocation_id: Uuid::new_v4(),
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
        let route_revision = self
            .route_revision
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                current.checked_add(ROUTE_REVISION_INCREMENT)
            })
            .map(|previous| previous + ROUTE_REVISION_INCREMENT)
            .expect(ROUTE_REVISION_EXHAUSTED);
        let record =
            CodexAdmissionRecord::capture_with_route_revision(&request, route_revision).unwrap();
        let artifacts = CodexHookArtifacts::new(
            &self.paths,
            self.terminal_id,
            self.runtime_epoch,
            identity.invocation_id,
        )
        .unwrap();
        let storage = CodexAdmissionStorage::for_identity(&artifacts, &identity).unwrap();
        let source = Arc::new(StableProcessSource {
            process: self.process,
        });
        let runtime = super::super::CodexHookRuntime::new_test(
            &self.paths,
            self.terminal_id,
            self.runtime_epoch,
            self.shell,
            identity.clone(),
            crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
            source,
        )
        .unwrap()
        .0;
        Authority {
            identity,
            runtime,
            request,
            record,
            storage,
        }
    }
}

impl Authority {
    pub(super) fn durable(&self) -> DurableCodexAdmission {
        let publication = self.storage.publish_for_test(&self.record).unwrap();
        DurableCodexAdmission::from_publication(self.storage.clone(), publication).unwrap()
    }

    pub(in crate::worker::codex_hook) fn install(
        &self,
        router: &CodexSessionRouter,
    ) -> Result<super::super::CodexRouteLease, super::super::CodexRouterError> {
        router.install_or_replay(self.durable(), |_| Ok(Arc::clone(&self.runtime)))
    }
}

pub(in crate::worker::codex_hook) fn ready_retirement(
    result: Result<CodexRetirementReadiness, super::super::CodexRouterError>,
) -> CodexRetirementPermit {
    match result.unwrap() {
        CodexRetirementReadiness::Ready(permit) => permit,
        CodexRetirementReadiness::Pending | CodexRetirementReadiness::Missing => unreachable!(),
    }
}
