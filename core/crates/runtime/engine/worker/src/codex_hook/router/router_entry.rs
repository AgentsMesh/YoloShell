use super::CodexHookRuntime;
use super::admission::DurableCodexAdmission;
use super::key::CodexRouteKey;
use crate::codex_hooks::CodexAdmissionRecord;
use std::sync::Arc;

pub(in crate::worker::codex_hook) struct CodexRouteEntry {
    key: CodexRouteKey,
    runtime: Arc<CodexHookRuntime>,
    admission: DurableCodexAdmission,
}

impl CodexRouteEntry {
    pub(super) fn new(
        key: CodexRouteKey,
        runtime: Arc<CodexHookRuntime>,
        admission: DurableCodexAdmission,
    ) -> Self {
        Self {
            key,
            runtime,
            admission,
        }
    }

    pub(in crate::worker::codex_hook) fn key(&self) -> CodexRouteKey {
        self.key
    }

    pub(in crate::worker::codex_hook) fn runtime(&self) -> &Arc<CodexHookRuntime> {
        &self.runtime
    }

    pub(in crate::worker::codex_hook) fn admission(&self) -> &DurableCodexAdmission {
        &self.admission
    }

    pub(in crate::worker::codex_hook) fn matches_record(
        &self,
        record: &CodexAdmissionRecord,
    ) -> bool {
        self.admission.record() == record
    }
}
