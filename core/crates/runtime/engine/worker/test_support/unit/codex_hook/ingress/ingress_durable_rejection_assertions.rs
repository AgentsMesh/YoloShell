use crate::agent::hooks::{
    AUTHORITY_REVISION_STEP, AttentionState, EventSequence, HookEnvelope, HookViolation,
    InstrumentationState, NormalizedLifecycle, ProcessState, HookProviderKind, Reducer, TurnState,
};
use crate::worker::codex_hook::runtime::CodexHookRuntime;
use std::path::Path;
use std::sync::Arc;

pub(super) fn assert_revoked(
    runtime: &Arc<CodexHookRuntime>,
    event: &HookEnvelope,
    violation: HookViolation,
) {
    let state = runtime.state().unwrap().unwrap();
    assert_revoked_state(&state, event, violation);
}

pub(super) fn assert_snapshot_revision(
    path: &Path,
    event: &HookEnvelope,
    violation: HookViolation,
) {
    let (_, snapshot) = crate::codex_hooks::CodexSnapshotCursor::open(path).unwrap();
    let reducer = Reducer::restore(
        snapshot.unwrap(),
        event.identity.clone(),
        HookProviderKind::Codex,
        event.provider_version,
    )
    .unwrap();
    let expected_revision = Reducer::new(
        event.identity.clone(),
        HookProviderKind::Codex,
        event.provider_version,
    )
    .unwrap()
    .revision()
    .checked_add(reducer.state().last_sequence.0 * AUTHORITY_REVISION_STEP)
    .unwrap()
    .checked_add(AUTHORITY_REVISION_STEP)
    .unwrap();
    assert_eq!(reducer.revision(), expected_revision);
    assert_revoked_state(reducer.state(), event, violation);
}

fn assert_revoked_state(
    state: &NormalizedLifecycle,
    event: &HookEnvelope,
    violation: HookViolation,
) {
    assert!(state.revoked);
    assert_eq!(state.process, ProcessState::Unknown);
    assert_eq!(state.turn, TurnState::Unknown);
    assert_eq!(state.attention, AttentionState::None);
    assert_eq!(state.instrumentation, InstrumentationState::Unavailable);
    assert_eq!(state.provider, HookProviderKind::Codex);
    assert_eq!(state.provider_version, event.provider_version);
    assert_eq!(state.generation, event.identity.generation);
    assert_eq!(state.expected_identity, event.identity);
    let accepted = event.sequence.0 - crate::agent::hooks::FIRST_SEQUENCE;
    if accepted == crate::agent::hooks::FIRST_SEQUENCE {
        let first = state.history.get(&EventSequence::FIRST).unwrap();
        assert_eq!(state.provider_session_id.as_ref(), Some(&first.provider_session_id));
        assert_eq!(state.current_cwd, first.metadata.cwd);
    } else {
        assert!(state.provider_session_id.is_none());
        assert!(state.current_cwd.is_none());
    }
    assert!(state.current_transcript_path.is_none());
    assert_eq!(state.last_sequence, EventSequence(accepted));
    assert!(state.stop_candidate.is_none());
    assert!(state.termination_candidate.is_none());
    assert!(!state.final_hook_seen);
    assert!(state.active_tools.is_empty());
    assert!(state.children.is_empty());
    assert_eq!(state.history.len(), accepted as usize);
    assert_eq!(state.event_ids.len(), accepted as usize);
    assert!(state.compaction.is_none());
    assert!(state.child_compactions.is_empty());
    let (rejected, proof) = state
        .revocation
        .as_ref()
        .and_then(|evidence| evidence.hook_rejection())
        .expect("hook revocation evidence");
    assert_eq!(rejected, event);
    assert_eq!(proof.event_id(), event.event_id);
    assert_eq!(proof.sequence(), event.sequence);
    assert_eq!(proof.violation(), violation);
}
