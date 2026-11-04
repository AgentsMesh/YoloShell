use super::super::error::CodexHookRejection;
use super::{CodexHookRuntime, CommittedLiveEvent, VerifiedLiveOccurrence};
use crate::agent::hooks::HookEnvelope;

pub(in crate::worker::codex_hook) struct CodexLiveFailure {
    assigned: Option<HookEnvelope>,
    rejection: CodexHookRejection,
    durable_rejection: Option<crate::codex_hooks::CodexDurableRejection>,
}

impl CodexLiveFailure {
    pub(super) fn uncommitted(rejection: CodexHookRejection) -> Self {
        Self {
            assigned: None,
            rejection,
            durable_rejection: None,
        }
    }

    pub(super) fn latched_commit_indeterminate() -> Self {
        Self::uncommitted(CodexHookRejection::CommitIndeterminate(anyhow::anyhow!(
            super::super::constants::CODEX_COMMIT_INDETERMINATE_ERROR
        )))
    }

    pub(super) fn after_preflight(
        runtime: &CodexHookRuntime,
        occurrence: &HookEnvelope,
        rejection: CodexHookRejection,
    ) -> Self {
        if rejection.is_commit_indeterminate()
            || matches!(&rejection, CodexHookRejection::Identity(_))
        {
            return Self::uncommitted(rejection);
        }
        match authoritative_outcome(runtime, occurrence) {
            Ok(Some(outcome)) => {
                let (assigned, durable_rejection) = outcome.into_parts();
                Self {
                    assigned: Some(assigned),
                    rejection,
                    durable_rejection,
                }
            }
            Ok(None) => Self::uncommitted(rejection),
            Err(error) => Self::uncommitted(CodexHookRejection::Storage(error)),
        }
    }

    pub(super) fn committed(event: &CommittedLiveEvent, rejection: CodexHookRejection) -> Self {
        Self {
            assigned: Some(event.event.clone()),
            rejection,
            durable_rejection: event.durable_rejection,
        }
    }

    pub(in crate::worker::codex_hook) fn event<'a>(
        &'a self,
        occurrence: &'a HookEnvelope,
    ) -> &'a HookEnvelope {
        self.assigned.as_ref().unwrap_or(occurrence)
    }

    pub(in crate::worker::codex_hook) fn code(&self) -> &'static str {
        self.durable_rejection.map_or_else(
            || self.rejection.code(),
            |marker| CodexHookRejection::Violation(marker.violation()).code(),
        )
    }

    pub(in crate::worker::codex_hook) fn rejection(&self) -> &CodexHookRejection {
        &self.rejection
    }

    pub(in crate::worker::codex_hook) fn is_commit_indeterminate(&self) -> bool {
        self.rejection.is_commit_indeterminate()
    }
}

fn authoritative_outcome(
    runtime: &CodexHookRuntime,
    occurrence: &HookEnvelope,
) -> anyhow::Result<Option<crate::codex_hooks::CodexAuthoritativeOccurrence>> {
    let Some(occurrence) = VerifiedLiveOccurrence::optional(runtime, occurrence)? else {
        return Ok(None);
    };
    occurrence.authoritative_outcome(&runtime.outbox)
}
