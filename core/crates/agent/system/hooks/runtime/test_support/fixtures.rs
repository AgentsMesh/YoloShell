use super::*;
use uuid::Uuid;

pub(super) const SESSION: &str = "provider-session";
pub(super) const CHILD: &str = "child-1";
pub(super) const TURN: &str = "turn-1";
pub(super) const TOOL: &str = "tool-1";
pub(super) const CHILD_TOOL: &str = "child-tool";
pub(super) const CODEX_VERSION: ProviderVersion = ProviderVersion::new(0, 150, 1);
pub(super) const CLAUDE_VERSION: ProviderVersion = ProviderVersion::new(2, 1, 241);
pub(super) const FIRST_POST_START_SEQUENCE: u64 = FIRST_SEQUENCE + 1;

pub(super) fn identity() -> HookIdentity {
    HookIdentity {
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        invocation_id: Uuid::new_v4(),
        hook_token: Uuid::new_v4(),
        process: ProcessIdentity::new(std::process::id(), 1),
        generation: Generation::INITIAL,
    }
}

pub(super) fn envelope(
    identity: &HookIdentity,
    version: ProviderVersion,
    sequence: u64,
    event: EventKind,
) -> HookEnvelope {
    HookEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        event_id: Uuid::new_v4(),
        sequence: EventSequence(sequence),
        provider: HookProviderKind::Codex,
        provider_version: version,
        identity: identity.clone(),
        provider_session_id: SESSION.to_owned(),
        turn_id: None,
        agent_id: None,
        tool_name: None,
        tool_use_id: None,
        stop_hook_active: None,
        metadata: Default::default(),
        event,
    }
}

pub(super) fn started(reducer: &mut Reducer, identity: &HookIdentity) {
    let start = envelope(
        identity,
        CODEX_VERSION,
        FIRST_SEQUENCE,
        EventKind::SessionStart {
            source: SessionStartSource::Startup,
        },
    );
    assert_eq!(reducer.apply(start).unwrap(), ApplyDisposition::Applied);
}

pub(super) fn claude_envelope(
    identity: &HookIdentity,
    sequence: u64,
    event: EventKind,
) -> HookEnvelope {
    let mut envelope = envelope(identity, CLAUDE_VERSION, sequence, event);
    envelope.provider = HookProviderKind::Claude;
    envelope
}

pub(super) fn claude_started(reducer: &mut Reducer, identity: &HookIdentity) {
    let start = claude_envelope(
        identity,
        FIRST_SEQUENCE,
        EventKind::SessionStart {
            source: SessionStartSource::Startup,
        },
    );
    assert_eq!(reducer.apply(start).unwrap(), ApplyDisposition::Applied);
}

pub(super) fn assert_stale_bookkeeping(
    before: &NormalizedLifecycle,
    after: &NormalizedLifecycle,
    event: &HookEnvelope,
) {
    let mut expected = before.clone();
    expected.last_sequence = event.sequence;
    expected.event_ids.insert(event.event_id, event.sequence);
    expected.history.insert(event.sequence, event.clone());
    assert_eq!(after, &expected);
}
