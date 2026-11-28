use crate::codex_hooks::{CodexNativeRequest, NATIVE_SCHEMA_VERSION};

#[path = "admission.rs"]
mod admission;
pub(super) struct Fixture {
    base: admission::Fixture,
    pub(super) paths: crate::paths::Paths,
    pub(super) identity: crate::agent::hooks::HookIdentity,
    pub(super) artifacts: crate::codex_hooks::CodexHookArtifacts,
    pub(super) request: crate::codex_hooks::CodexAdmissionContext,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let base = admission::Fixture::new();
        Self {
            paths: base.paths.clone(),
            identity: base.identity.clone(),
            artifacts: base.artifacts.clone(),
            request: base.request.clone(),
            base,
        }
    }

    pub(super) fn record(&self) -> crate::codex_hooks::CodexAdmissionRecord {
        self.base.record()
    }

    pub(super) fn storage(&self) -> crate::codex_hooks::CodexAdmissionStorage {
        self.base.storage()
    }
}

pub(super) const AGENT_SESSION: &str = "native-codex-session";
pub(super) const MODEL: &str = "test-model";
pub(super) const PROMPT: &str = "test prompt";
pub(super) const TURN: &str = "test-turn";
pub(super) const INITIAL_ROUTE: u64 = 1;
pub(super) const CONFLICTING_FILE: &str = "replaced-request";

pub(super) fn request(fixture: &Fixture) -> CodexNativeRequest {
    CodexNativeRequest {
        schema_version: NATIVE_SCHEMA_VERSION,
        terminal_id: fixture.identity.terminal_id,
        runtime_epoch: fixture.identity.runtime_epoch,
        event_id: uuid::Uuid::new_v4(),
        hook_executable: fixture.request.hook_executable.clone(),
        data_home_candidates: vec![fixture.request.data_home.clone()],
        input: crate::codex_hooks::CodexHookInput::SessionStart {
            session_id: AGENT_SESSION.to_owned(),
            transcript_path: None,
            cwd: fixture.request.cwd.clone(),
            model: MODEL.to_owned(),
            permission_mode: crate::codex_hooks::constants::permission_modes::DEFAULT.to_owned(),
            source: crate::codex_hooks::constants::values::STARTUP.to_owned(),
        },
    }
}

pub(super) fn prompt(fixture: &Fixture) -> CodexNativeRequest {
    let mut request = request(fixture);
    request.input = crate::codex_hooks::CodexHookInput::UserPromptSubmit {
        session_id: AGENT_SESSION.to_owned(),
        transcript_path: None,
        cwd: fixture.request.cwd.clone(),
        model: MODEL.to_owned(),
        permission_mode: crate::codex_hooks::constants::permission_modes::DEFAULT.to_owned(),
        turn_id: TURN.to_owned(),
        prompt: PROMPT.to_owned(),
        agent_id: None,
        agent_type: None,
    };
    request
}
