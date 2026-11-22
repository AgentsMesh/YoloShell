use crate::codex_hooks::{
    CodexAdmissionContext, CodexAdmissionPublication, CodexAdmissionRecord, CodexAdmissionStorage,
    CodexHookArtifacts, CodexRetirementArtifact, CodexRetirementRecord,
};
use anyhow::Result;
use std::path::PathBuf;

const TEST_ROUTE_REVISION: u64 = 1;

impl CodexAdmissionStorage {
    pub fn publish_for_test(
        &self,
        record: &CodexAdmissionRecord,
    ) -> Result<CodexAdmissionPublication> {
        self.publish(record)
    }
}

impl CodexAdmissionRecord {
    pub fn capture_for_test(request: &CodexAdmissionContext) -> Result<Self> {
        Self::capture_with_route_revision(request, TEST_ROUTE_REVISION)
    }

    pub fn capture_with_route_revision(
        request: &CodexAdmissionContext,
        revision: u64,
    ) -> Result<Self> {
        use crate::agent::hooks::{
            EventKind, EventSequence, HookEnvelope, HookMetadata, HookProviderKind, SessionStartSource,
        };
        let first = HookEnvelope {
            schema_version: crate::agent::hooks::CURRENT_SCHEMA_VERSION,
            event_id: request.identity.invocation_id,
            sequence: EventSequence::ZERO,
            provider: HookProviderKind::Codex,
            provider_version: request.provider_version,
            identity: request.identity.clone(),
            provider_session_id: request.identity.invocation_id.to_string(),
            turn_id: None,
            agent_id: None,
            tool_name: None,
            tool_use_id: None,
            stop_hook_active: None,
            metadata: HookMetadata {
                cwd: Some(request.cwd.clone()),
                transcript_path: None,
            },
            event: EventKind::SessionStart {
                source: SessionStartSource::Startup,
            },
        };
        Self::capture_native_with_route_revision(request.clone(), revision, first)
    }

    #[cfg(test)]
    pub(crate) fn capture(request: &CodexAdmissionContext) -> Result<Self> {
        Self::capture_for_test(request)
    }
}

impl CodexHookArtifacts {
    pub fn retirement_path_for_test(&self) -> PathBuf {
        self.retirement_path()
    }

    pub fn retirement_seal_path_for_test(&self) -> PathBuf {
        self.retirement_seal_path()
    }

    pub fn retirement_writer_lock_path_for_test(&self) -> PathBuf {
        self.retirement_writer_lock_path()
    }
}

impl CodexRetirementArtifact {
    pub fn record_for_test(&self) -> &CodexRetirementRecord {
        self.record()
    }
}
