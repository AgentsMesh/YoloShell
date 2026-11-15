use super::super::router::test_support::support::{Authority, Fixture};
use super::super::router::{CodexCommittedRoute, CodexSessionRouter};
use crate::agent::hooks::test_values::{TOOL_NAME, TOOL_USE_ID};
use crate::agent::hooks::{
    EventKind, EventSequence, HookEnvelope, HookMetadata, NormalizedLifecycle, HookProviderKind,
    SessionEndReason, SessionStartSource,
};
use uuid::Uuid;

#[path = "projection_transcript.rs"]
mod transcript;

pub(super) const FIRST_SEQUENCE: u64 = crate::agent::hooks::FIRST_SEQUENCE;
pub(super) const SECOND_SEQUENCE: u64 = FIRST_SEQUENCE + 1;
pub(super) const THIRD_SEQUENCE: u64 = SECOND_SEQUENCE + 1;
pub(super) const FOURTH_SEQUENCE: u64 = THIRD_SEQUENCE + 1;
pub(super) const FIFTH_SEQUENCE: u64 = FOURTH_SEQUENCE + 1;
pub(super) const PROVIDER_SESSION: &str = "550e8400-e29b-41d4-a716-446655440000";
pub(super) const TRANSCRIPT_FILE: &str =
    "rollout-2026-08-29T00-00-00-550e8400-e29b-41d4-a716-446655440000.jsonl";

pub(super) fn registry() -> crate::agent::AgentRegistry {
    crate::agent::agent_registry_from_runtime(
        &yolo_agent_providers::builtin_provider_runtime(),
    )
}
pub(super) struct ProjectionFixture {
    pub(super) _filesystem: Fixture,
    pub(super) router: CodexSessionRouter,
    pub(super) authority: Option<Authority>,
    pub(super) committed: Option<CodexCommittedRoute>,
}

impl ProjectionFixture {
    pub(super) fn new() -> Self {
        let filesystem = Fixture::new();
        let router = filesystem.router();
        let authority = filesystem.authority();
        authority.install(&router).unwrap().settle().unwrap();
        let committed = router
            .subscribe_committed()
            .unwrap()
            .snapshot
            .into_iter()
            .next()
            .unwrap();
        Self {
            _filesystem: filesystem,
            router,
            authority: Some(authority),
            committed: Some(committed),
        }
    }

    pub(super) fn route(&self) -> super::route::ProjectionRoute {
        super::route::ProjectionRoute::capture(self.committed.as_ref().unwrap())
    }

    pub(super) fn start(&self) -> NormalizedLifecycle {
        let authority = self.authority.as_ref().unwrap();
        let metadata = HookMetadata {
            cwd: Some(authority.request.cwd.clone()),
            transcript_path: Some(transcript::write(authority)),
        };
        submit_event(
            authority,
            FIRST_SEQUENCE,
            EventKind::SessionStart {
                source: SessionStartSource::Startup,
            },
            metadata,
            None,
        )
    }

    pub(super) fn prompt(&self) -> NormalizedLifecycle {
        submit_event(
            self.authority.as_ref().unwrap(),
            SECOND_SEQUENCE,
            EventKind::UserPromptSubmit,
            HookMetadata::default(),
            None,
        )
    }

    pub(super) fn stop(&self) -> NormalizedLifecycle {
        submit_event(
            self.authority.as_ref().unwrap(),
            THIRD_SEQUENCE,
            EventKind::Stop,
            HookMetadata::default(),
            Some(false),
        )
    }

    pub(super) fn pre_compact(&self) -> NormalizedLifecycle {
        submit_event(
            self.authority.as_ref().unwrap(),
            THIRD_SEQUENCE,
            EventKind::PreCompact,
            HookMetadata::default(),
            None,
        )
    }

    pub(super) fn finish(&self) -> NormalizedLifecycle {
        submit_event(
            self.authority.as_ref().unwrap(),
            FOURTH_SEQUENCE,
            EventKind::SessionEnd {
                reason: SessionEndReason::Other,
            },
            HookMetadata::default(),
            None,
        )
    }

    pub(super) fn confirm_exit(&self) {
        let authority = self.authority.as_ref().unwrap();
        let publication = authority
            .runtime
            .observe_process_exit(authority.identity.process)
            .unwrap();
        assert_eq!(
            publication.disposition,
            crate::agent::hooks::ProcessExitDisposition::Confirmed
        );
    }
}

pub(super) fn submit_event(
    authority: &Authority,
    sequence: u64,
    event: EventKind,
    metadata: HookMetadata,
    stop_hook_active: Option<bool>,
) -> NormalizedLifecycle {
    let (tool_name, tool_use_id) = match event {
        EventKind::PermissionRequest => (Some(TOOL_NAME.to_owned()), None),
        EventKind::PreToolUse | EventKind::PostToolUse => {
            (Some(TOOL_NAME.into()), Some(TOOL_USE_ID.into()))
        }
        _ => (None, None),
    };
    let envelope = HookEnvelope {
        schema_version: crate::agent::hooks::CURRENT_SCHEMA_VERSION,
        event_id: Uuid::new_v4(),
        sequence: EventSequence(sequence),
        provider: HookProviderKind::Codex,
        provider_version: crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
        identity: authority.identity.clone(),
        provider_session_id: PROVIDER_SESSION.to_owned(),
        turn_id: matches!(
            &event,
            EventKind::UserPromptSubmit
                | EventKind::PreToolUse
                | EventKind::PostToolUse
                | EventKind::PermissionRequest
                | EventKind::PreCompact
                | EventKind::Stop
        )
        .then(|| PROVIDER_SESSION.to_owned()),
        agent_id: None,
        tool_name,
        tool_use_id,
        stop_hook_active,
        metadata,
        event,
    };
    let outbox = crate::codex_hooks::CodexOutbox::for_identity(
        authority.runtime.outbox_path(),
        &authority.identity,
    )
    .unwrap();
    let envelope = outbox.enqueue(envelope).unwrap();
    let peer_pid = authority.identity.process.pid;
    let publication = authority
        .runtime
        .ingest(envelope.clone(), peer_pid)
        .unwrap();
    outbox
        .acknowledge(envelope.sequence.0, envelope.event_id)
        .unwrap();
    publication.state
}
