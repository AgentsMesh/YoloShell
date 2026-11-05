use crate::agent::hooks::{Generation, HookIdentity, ProcessIdentity};
use crate::codex_hooks::{
    CodexAdmissionAuthority, CodexAdmissionContext, CodexAdmissionRecord, CodexAdmissionStorage,
    CodexHookArtifacts, CodexHookInput, CodexNativeRequest, CodexOutbox,
};
use uuid::Uuid;

#[path = "native_registration_process.rs"]
mod process_source;

const TEST_HOME: &str = "home";
const TEST_MODEL: &str = "gpt-5.4";
const TEST_PERMISSION: &str = "default";
const TEST_STARTUP: &str = "startup";
const TEST_ROUTE_REVISION: u64 = 1;

pub(super) struct Fixture {
    _root: tempfile::TempDir,
    pub(super) paths: crate::paths::Paths,
    pub(super) artifacts: CodexHookArtifacts,
    pub(super) admission: CodexAdmissionAuthority,
}

impl Fixture {
    pub(super) fn runtime(
        &self,
    ) -> std::sync::Arc<crate::worker::codex_hook::runtime::CodexHookRuntime> {
        super::prime(&self.paths, &self.admission).unwrap();
        let owner = self.admission.record().request().identity.clone();
        let process = crate::process::ProcessIdentity {
            pid: owner.process.pid,
            start_time: owner.process.start_time,
        };
        let parent = u32::try_from(unsafe { libc::getppid() }).unwrap();
        let shell = crate::process::ProcessIdentity::capture(parent).unwrap();
        crate::worker::codex_hook::runtime::CodexHookRuntime::new_test(
            &self.paths,
            owner.terminal_id,
            owner.runtime_epoch,
            shell,
            owner,
            crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
            std::sync::Arc::new(process_source::StableSource(process)),
        )
        .unwrap()
        .0
    }

    pub(super) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let cwd = root.path().canonicalize().unwrap();
        let executable = std::env::current_exe().unwrap().canonicalize().unwrap();
        let paths = crate::paths::Paths::new(cwd.join(TEST_HOME));
        let process = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
        let identity = HookIdentity {
            terminal_id: Uuid::new_v4(),
            runtime_epoch: Uuid::new_v4(),
            invocation_id: Uuid::new_v4(),
            hook_token: Uuid::new_v4(),
            generation: Generation::INITIAL,
            process: ProcessIdentity::new(process.pid, process.start_time),
        };
        paths.ensure_session(identity.terminal_id).unwrap();
        let request = CodexNativeRequest {
            schema_version: crate::codex_hooks::NATIVE_SCHEMA_VERSION,
            terminal_id: identity.terminal_id,
            runtime_epoch: identity.runtime_epoch,
            event_id: Uuid::new_v4(),
            hook_executable: executable.clone(),
            data_home_candidates: vec![cwd.clone()],
            input: CodexHookInput::SessionStart {
                session_id: Uuid::new_v4().to_string(),
                transcript_path: None,
                cwd: cwd.clone(),
                model: TEST_MODEL.to_owned(),
                permission_mode: TEST_PERMISSION.to_owned(),
                source: TEST_STARTUP.to_owned(),
            },
        };
        let first = request.envelope(&identity).unwrap();
        let context = CodexAdmissionContext {
            identity: identity.clone(),
            provider_version: crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
            cwd: cwd.clone(),
            data_home: cwd,
            hook_executable: executable.clone(),
            real_executable: executable,
        };
        let artifacts = CodexHookArtifacts::new(
            &paths,
            identity.terminal_id,
            identity.runtime_epoch,
            identity.invocation_id,
        )
        .unwrap();
        let storage = CodexAdmissionStorage::for_identity(&artifacts, &identity).unwrap();
        let record = CodexAdmissionRecord::capture_native_with_route_revision(
            context,
            TEST_ROUTE_REVISION,
            first,
        )
        .unwrap();
        let publication = storage
            .publish_with_commit_fence(&record, || Ok(()))
            .unwrap();
        let admission = CodexAdmissionAuthority::from_publication(storage, publication).unwrap();
        Self {
            _root: root,
            paths,
            artifacts,
            admission,
        }
    }

    pub(super) fn outbox(&self) -> CodexOutbox {
        CodexOutbox::for_identity(
            self.artifacts.outbox_path(),
            &self.admission.record().request().identity,
        )
        .unwrap()
    }
}
