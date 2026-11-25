use super::super::runtime::{CodexHookPublication, CodexHookRuntime};
pub(super) use super::transport_support::{
    exchange, exchange_uncommitted, read_ack, saturate_ack_channel, write_bytes_frame,
};
use crate::agent::hooks::{
    EventKind, EventSequence, Generation, HookEnvelope, HookIdentity, ProcessIdentity,
    HookProviderKind, SessionStartSource,
};
use std::sync::Arc;
use uuid::Uuid;

pub(super) const TEST_PROVIDER_SESSION: &str = "codex-provider-session";
pub(super) const FIRST_EVENT_SEQUENCE: u64 = crate::agent::hooks::FIRST_SEQUENCE;
pub(super) const SECOND_EVENT_SEQUENCE: u64 = FIRST_EVENT_SEQUENCE + SEQUENCE_INCREMENT;
pub(super) const GAP_EVENT_SEQUENCE: u64 = SECOND_EVENT_SEQUENCE + SEQUENCE_INCREMENT;
pub(super) const UNASSIGNED_EVENT_SEQUENCE: u64 = crate::agent::hooks::ZERO_SEQUENCE;
pub(super) const FRAME_LENGTH_OVERFLOW: usize = 1;
pub(super) const INVALID_PROCESS_START_DELTA: u64 = 1;
pub(super) const EMPTY_EVENT_COUNT: usize = 0;
pub(super) const SINGLE_EVENT_COUNT: usize = 1;
const SEQUENCE_INCREMENT: u64 = 1;
const SESSION_UUID: u128 = 0x10000000000000000000000000000001;
const RUNTIME_UUID: u128 = 0x20000000000000000000000000000002;
const ANOTHER_INVOCATION_UUID: u128 = 0x30000000000000000000000000000004;
const TOKEN_UUID: u128 = 0x40000000000000000000000000000004;
const START_EVENT_UUID: u128 = 0x50000000000000000000000000000005;
const PROMPT_EVENT_UUID: u128 = 0x60000000000000000000000000000006;
const OTHER_EVENT_UUID: u128 = 0x70000000000000000000000000000007;
const INVALID_UUID_DELTA: u128 = 1;
const TEST_HOME_DIRECTORY: &str = "worker-codex-hook";
pub(super) struct Fixture {
    _directory: tempfile::TempDir,
    pub paths: crate::paths::Paths,
    pub runtime: Arc<CodexHookRuntime>,
    pub replay: super::super::runtime::ReplayReport,
    pub identity: HookIdentity,
    pub shell: crate::process::ProcessIdentity,
    pub process: crate::process::ProcessIdentity,
}

include!("support_fixture_construction.rs");

impl Fixture {
    pub(super) const fn terminal_id() -> Uuid {
        Uuid::from_u128(SESSION_UUID)
    }

    pub(super) const fn runtime_epoch() -> Uuid {
        Uuid::from_u128(RUNTIME_UUID)
    }

    pub(super) fn start(&self, sequence: u64) -> HookEnvelope {
        envelope(
            self.identity.clone(),
            sequence,
            Uuid::from_u128(START_EVENT_UUID),
            EventKind::SessionStart {
                source: SessionStartSource::Startup,
            },
        )
    }

    pub(super) fn prompt(&self, sequence: u64) -> HookEnvelope {
        envelope(
            self.identity.clone(),
            sequence,
            Uuid::from_u128(PROMPT_EVENT_UUID),
            EventKind::UserPromptSubmit,
        )
    }

    pub(super) fn other_event_id() -> Uuid {
        Uuid::from_u128(OTHER_EVENT_UUID)
    }

    pub(super) fn another_session() -> Uuid {
        Uuid::from_u128(SESSION_UUID + INVALID_UUID_DELTA)
    }

    pub(super) fn another_runtime_epoch() -> Uuid {
        Uuid::from_u128(RUNTIME_UUID + INVALID_UUID_DELTA)
    }

    pub(super) fn another_invocation() -> Uuid {
        Uuid::from_u128(ANOTHER_INVOCATION_UUID)
    }

    pub(super) fn enqueue(&self, event: HookEnvelope) -> HookEnvelope {
        crate::codex_hooks::CodexOutbox::for_identity(self.runtime.outbox_path(), &self.identity)
            .unwrap()
            .enqueue(event)
            .unwrap()
    }

    pub(super) fn submit(
        &self,
        event: HookEnvelope,
    ) -> Result<CodexHookPublication, super::super::error::CodexHookRejection> {
        let event = self.enqueue(event);
        let result = self.runtime.ingest(event.clone(), self.process.pid);
        if result.is_ok() {
            crate::codex_hooks::CodexOutbox::for_identity(
                self.runtime.outbox_path(),
                &self.identity,
            )
            .unwrap()
            .acknowledge(event.sequence.0, event.event_id)
            .unwrap();
        }
        result
    }

    pub(super) fn restart_runtime(
        &self,
    ) -> (Arc<CodexHookRuntime>, super::super::runtime::ReplayReport) {
        self.try_restart_runtime().unwrap()
    }

    pub(super) fn try_restart_runtime(
        &self,
    ) -> anyhow::Result<(Arc<CodexHookRuntime>, super::super::runtime::ReplayReport)> {
        CodexHookRuntime::new_test(
            &self.paths,
            self.identity.terminal_id,
            self.identity.runtime_epoch,
            self.shell,
            self.identity.clone(),
            crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
            self.process_source(),
        )
    }

    pub(super) fn process_source(&self) -> Arc<dyn crate::agent::ProcessSource> {
        super::process_support::stub_process_source(self.process)
    }
}

pub(super) fn envelope(
    identity: HookIdentity,
    sequence: u64,
    event_id: Uuid,
    event: EventKind,
) -> HookEnvelope {
    HookEnvelope {
        schema_version: crate::agent::hooks::CURRENT_SCHEMA_VERSION,
        event_id,
        sequence: EventSequence(sequence),
        provider: HookProviderKind::Codex,
        provider_version: crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
        identity,
        provider_session_id: TEST_PROVIDER_SESSION.to_owned(),
        turn_id: matches!(&event, EventKind::UserPromptSubmit)
            .then(|| TEST_PROVIDER_SESSION.into()),
        agent_id: None,
        tool_name: None,
        tool_use_id: None,
        stop_hook_active: None,
        metadata: Default::default(),
        event,
    }
}
