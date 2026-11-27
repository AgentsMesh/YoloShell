use super::*;
use crate::test_support::{
    CHILD, CLAUDE_VERSION, CODEX_VERSION, SESSION, claude_envelope, envelope, identity, started,
};
use crate::{ChildLifecycle, Generation, Reducer, TurnState};
use uuid::Uuid;

const STEP: u64 = 1;
const FOREIGN_SESSION: &str = "foreign-session";

#[test]
fn envelope_identity_rejections_identify_the_exact_broken_contract() {
    let identity = identity();
    let reducer = Reducer::new(identity.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    let original = envelope(
        &identity,
        CODEX_VERSION,
        crate::FIRST_SEQUENCE,
        EventKind::UserPromptSubmit,
    );
    validate_envelope(reducer.state(), &original).unwrap();
    let mutations: &[(&str, HookViolation, fn(&mut HookEnvelope))] = &[
        (
            stringify!(schema),
            HookViolation::InvalidEnvelope,
            |value| value.schema_version += 1,
        ),
        (
            stringify!(event_id),
            HookViolation::InvalidEnvelope,
            |value| value.event_id = Uuid::nil(),
        ),
        (
            stringify!(identity_nil),
            HookViolation::InvalidIdentity,
            |value| value.identity.terminal_id = Uuid::nil(),
        ),
        (
            stringify!(terminal_id),
            HookViolation::InvalidIdentity,
            |value| value.identity.terminal_id = Uuid::new_v4(),
        ),
        (
            stringify!(runtime_epoch),
            HookViolation::InvalidIdentity,
            |value| value.identity.runtime_epoch = Uuid::new_v4(),
        ),
        (
            stringify!(invocation_id),
            HookViolation::InvalidIdentity,
            |value| value.identity.invocation_id = Uuid::new_v4(),
        ),
        (
            stringify!(hook_token),
            HookViolation::InvalidIdentity,
            |value| value.identity.hook_token = Uuid::new_v4(),
        ),
        (
            stringify!(generation),
            HookViolation::GenerationMismatch,
            |value| value.identity.generation = Generation(value.identity.generation.0 + STEP),
        ),
        (
            stringify!(process),
            HookViolation::ProcessMismatch,
            |value| value.identity.process.start_time += STEP,
        ),
        (
            stringify!(provider),
            HookViolation::ProviderMismatch,
            |value| value.provider = HookProviderKind::Claude,
        ),
        (
            stringify!(version),
            HookViolation::VersionMismatch,
            |value| value.provider_version.patch += 1,
        ),
        (
            stringify!(bounded_size),
            HookViolation::InvalidEnvelope,
            |value| value.provider_session_id = "s".repeat(crate::MAX_HOOK_IDENTIFIER_BYTES + 1),
        ),
        (
            stringify!(metadata),
            HookViolation::InvalidEnvelope,
            |value| value.metadata.cwd = Some("relative".into()),
        ),
    ];
    for (field, expected, mutate) in mutations {
        let mut changed = original.clone();
        mutate(&mut changed);
        assert_eq!(
            validate_envelope(reducer.state(), &changed),
            Err(*expected),
            "{field}"
        );
    }
}

#[test]
fn provider_contract_accepts_codex_hooks_but_not_claude_only_events() {
    let identity = identity();
    let codex = Reducer::new(identity.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    let claude = Reducer::new(identity.clone(), HookProviderKind::Claude, CLAUDE_VERSION).unwrap();
    for event in [
        EventKind::SessionStart {
            source: SessionStartSource::Fork,
        },
        EventKind::SessionEnd {
            reason: SessionEndReason::Clear,
        },
        EventKind::SessionEnd {
            reason: SessionEndReason::Resume,
        },
        EventKind::PermissionDenied,
        EventKind::PostToolUseFailure,
    ] {
        let request = envelope(
            &identity,
            CODEX_VERSION,
            crate::FIRST_SEQUENCE,
            event.clone(),
        );
        assert_eq!(
            validate_envelope(codex.state(), &request),
            Err(HookViolation::UnsupportedEvent)
        );
        let request = claude_envelope(&identity, crate::FIRST_SEQUENCE, event);
        validate_envelope(claude.state(), &request).unwrap();
    }
}

#[path = "admission_topology_matrix_tests.rs"]
mod admission_topology_matrix_tests;
