use super::super::constants::{FIRST_SEQUENCE, OUTBOX_FILE_NAME, outbox_fields, test_values};
use super::{envelope, identity};
use crate::agent::hooks::{
    EventKind, EventSequence, HookViolation, MAX_HOOK_IDENTIFIER_BYTES, NotificationKind,
    HookProviderKind, ProviderVersion,
};
use crate::codex_hooks::{CodexOutbox, CodexOutboxState};
use std::path::PathBuf;
use tempfile::TempDir;
use uuid::Uuid;

const UNSUPPORTED_SCHEMA_VERSION: u16 = u16::MAX;
const UNSUPPORTED_PROVIDER_VERSION: ProviderVersion = ProviderVersion::new(0, 150, 0);
const IDENTIFIER_OVERFLOW_BYTES: usize = 1;
const INVALID_METADATA_PATH: &str = "relative/metadata";

#[test]
fn persisted_envelopes_require_current_codex_contract() {
    let mut wrong_schema = pending_state();
    wrong_schema.pending[0].schema_version = UNSUPPORTED_SCHEMA_VERSION;
    assert_rejected(wrong_schema);

    let mut wrong_provider = pending_state();
    wrong_provider.pending[0].provider = HookProviderKind::Claude;
    assert_rejected(wrong_provider);

    let mut unsupported_event = pending_state();
    unsupported_event.pending[0].event = EventKind::StopFailure;
    assert_rejected(unsupported_event);

    let mut invalid_shape = pending_state();
    invalid_shape.pending[0].turn_id = None;
    assert_rejected(invalid_shape);
}

#[test]
fn rejected_enqueue_does_not_bind_or_advance_the_outbox() {
    let root = tempfile::tempdir().unwrap();
    let outbox = CodexOutbox::new(root.path().join(OUTBOX_FILE_NAME)).unwrap();
    let owner = identity();
    let mut invalid = envelope(&owner, Uuid::new_v4());
    invalid.provider = HookProviderKind::Claude;

    assert!(outbox.enqueue(invalid).is_err());
    assert_eq!(outbox.load().unwrap(), CodexOutboxState::default());
}

#[test]
fn persisted_envelopes_reject_unknown_fields() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(OUTBOX_FILE_NAME);
    let mut value = serde_json::to_value(pending_state()).unwrap();
    value[outbox_fields::PENDING][0][test_values::UNKNOWN_FIELD] = serde_json::json!(true);
    crate::paths::atomic_write_json(&path, &value).unwrap();
    assert!(CodexOutbox::new(path).is_err());
}

#[test]
fn rejected_validation_accepts_only_bounded_reducer_mismatches() {
    let owner = identity();

    let mut provider_mismatch = envelope(&owner, Uuid::new_v4());
    provider_mismatch.provider = HookProviderKind::Claude;
    assert_relaxed_accepts(provider_mismatch, HookViolation::ProviderMismatch);

    let mut version_mismatch = envelope(&owner, Uuid::new_v4());
    version_mismatch.provider_version = UNSUPPORTED_PROVIDER_VERSION;
    assert_relaxed_accepts(version_mismatch, HookViolation::VersionMismatch);

    for event in [
        EventKind::Notification {
            kind: NotificationKind::AgentNeedsInput,
        },
        EventKind::PostToolUseFailure,
    ] {
        let mut unsupported = envelope(&owner, Uuid::new_v4());
        unsupported.event = event;
        assert_relaxed_accepts(unsupported, HookViolation::UnsupportedEvent);
    }

    let mut missing_agent = envelope(&owner, Uuid::new_v4());
    missing_agent.event = EventKind::SubagentStart;
    assert_relaxed_accepts(missing_agent, HookViolation::InvalidTopology);

    let mut existing_agent = envelope(&owner, Uuid::new_v4());
    existing_agent.event = EventKind::SubagentStart;
    existing_agent.agent_id = Some(test_values::AGENT_ID.to_owned());
    assert_relaxed_accepts(existing_agent, HookViolation::InvalidTopology);
}

#[test]
fn rejected_validation_remains_fail_closed_for_wrong_contracts() {
    let owner = identity();

    let mut wrong_schema = envelope(&owner, Uuid::new_v4());
    wrong_schema.schema_version = UNSUPPORTED_SCHEMA_VERSION;
    wrong_schema.provider = HookProviderKind::Claude;
    assert_relaxed_rejects(wrong_schema, HookViolation::ProviderMismatch);

    let mut unsupported = envelope(&owner, Uuid::new_v4());
    unsupported.event = EventKind::StopFailure;
    assert_relaxed_rejects(unsupported, HookViolation::UnsupportedEvent);

    let wrong_provider_violation = envelope(&owner, Uuid::new_v4());
    assert_relaxed_rejects(wrong_provider_violation, HookViolation::ProviderMismatch);

    let mut wrong_event_violation = envelope(&owner, Uuid::new_v4());
    wrong_event_violation.provider_version = UNSUPPORTED_PROVIDER_VERSION;
    assert_relaxed_rejects(wrong_event_violation, HookViolation::UnsupportedEvent);

    let mut invalid_metadata = envelope(&owner, Uuid::new_v4());
    invalid_metadata.metadata.cwd = Some(PathBuf::from(INVALID_METADATA_PATH));
    invalid_metadata.provider = HookProviderKind::Claude;
    assert_relaxed_rejects(invalid_metadata, HookViolation::ProviderMismatch);

    let mut unbounded_identifier = envelope(&owner, Uuid::new_v4());
    unbounded_identifier.provider_session_id = "x".repeat(
        MAX_HOOK_IDENTIFIER_BYTES
            .checked_add(IDENTIFIER_OVERFLOW_BYTES)
            .unwrap(),
    );
    unbounded_identifier.provider = HookProviderKind::Claude;
    assert_relaxed_rejects(unbounded_identifier, HookViolation::ProviderMismatch);

    let valid_event = envelope(&owner, Uuid::new_v4());
    assert_relaxed_rejects(valid_event, HookViolation::ProcessAlreadyExited);
}

#[test]
fn strict_validation_does_not_accept_reducer_only_shapes() {
    let owner = identity();

    let mut unsupported = envelope(&owner, Uuid::new_v4());
    unsupported.event = EventKind::Notification {
        kind: NotificationKind::AgentNeedsInput,
    };
    assert!(super::super::outbox_envelope_validation::validate(&unsupported).is_err());

    let mut missing_agent = envelope(&owner, Uuid::new_v4());
    missing_agent.event = EventKind::SubagentStart;
    assert!(super::super::outbox_envelope_validation::validate(&missing_agent).is_err());

    let nil_id = envelope(&owner, Uuid::nil());
    assert!(super::super::outbox_envelope_validation::validate(&nil_id).is_err());
}

fn pending_state() -> CodexOutboxState {
    let owner = identity();
    let mut state = CodexOutboxState {
        next_sequence: FIRST_SEQUENCE.checked_add(FIRST_SEQUENCE).unwrap(),
        owner: Some(owner.hook.clone()),
        ..CodexOutboxState::default()
    };
    let mut event = envelope(&owner, Uuid::new_v4());
    event.sequence = EventSequence::FIRST;
    state.pending.push_back(event);
    state
}

fn assert_rejected(state: CodexOutboxState) {
    let root = TempDir::new().unwrap();
    let path = root.path().join(OUTBOX_FILE_NAME);
    crate::paths::atomic_write_json(&path, &state).unwrap();
    assert!(CodexOutbox::new(path).is_err());
}

fn assert_relaxed_accepts(event: crate::agent::hooks::HookEnvelope, violation: HookViolation) {
    assert!(
        super::super::outbox_envelope_validation::validate_rejected(&event, violation).is_ok(),
        "bounded event should be accepted for {violation:?}"
    );
}

fn assert_relaxed_rejects(event: crate::agent::hooks::HookEnvelope, violation: HookViolation) {
    assert!(
        super::super::outbox_envelope_validation::validate_rejected(&event, violation).is_err(),
        "event should be rejected for {violation:?}"
    );
}
