use crate::test_support::{
    CHILD, CLAUDE_VERSION, CODEX_VERSION, claude_envelope, claude_started, envelope, identity,
    started,
};
use crate::*;

const SEQUENCE_STEP: u64 = 1;

#[test]
fn permission_request_needs_a_notification_to_confirm_the_prompt_in_each_scope() {
    for child in [false, true] {
        let identity = identity();
        let mut reducer =
            Reducer::new(identity.clone(), HookProviderKind::Claude, CLAUDE_VERSION).unwrap();
        claude_started(&mut reducer, &identity);
        let mut sequence = FIRST_SEQUENCE + SEQUENCE_STEP;
        if child {
            let mut start = claude_envelope(&identity, sequence, EventKind::SubagentStart);
            start.agent_id = Some(CHILD.into());
            reducer.apply(start).unwrap();
            sequence += SEQUENCE_STEP;
        }
        for (event, expected) in [
            (
                EventKind::PermissionRequest,
                AttentionState::PermissionCandidate,
            ),
            (
                EventKind::Notification {
                    kind: NotificationKind::PermissionPrompt,
                },
                AttentionState::Permission,
            ),
            (EventKind::PermissionDenied, AttentionState::None),
            (
                EventKind::PermissionRequest,
                AttentionState::PermissionCandidate,
            ),
            (EventKind::PreToolUse, AttentionState::None),
        ] {
            let mut event = claude_envelope(&identity, sequence, event);
            event.agent_id = child.then(|| CHILD.into());
            assert_eq!(
                reducer.apply(event.clone()).unwrap(),
                ApplyDisposition::Applied
            );
            let state = reducer.state().clone();
            let attention = if child {
                state.children[CHILD].attention
            } else {
                state.attention
            };
            assert_eq!(attention, expected);
            assert_eq!(reducer.apply(event).unwrap(), ApplyDisposition::Replay);
            assert_eq!(reducer.state(), &state);
            if child {
                assert_eq!(state.attention, AttentionState::None);
            }
            sequence += SEQUENCE_STEP;
        }
        Reducer::restore(
            reducer.snapshot().unwrap(),
            identity,
            HookProviderKind::Claude,
            CLAUDE_VERSION,
        )
        .unwrap();
    }
}

#[test]
fn codex_permission_candidate_survives_replay_and_restart_without_claiming_approval() {
    let identity = identity();
    let mut reducer = Reducer::new(identity.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    started(&mut reducer, &identity);
    let permission = envelope(
        &identity,
        CODEX_VERSION,
        FIRST_SEQUENCE + SEQUENCE_STEP,
        EventKind::PermissionRequest,
    );
    reducer.apply(permission.clone()).unwrap();
    let snapshot = reducer.snapshot().unwrap();
    let mut restored =
        Reducer::restore(snapshot, identity, HookProviderKind::Codex, CODEX_VERSION).unwrap();
    assert_eq!(
        restored.state().attention,
        AttentionState::PermissionCandidate
    );
    let revision = restored.revision();
    assert_eq!(
        restored.apply(permission).unwrap(),
        ApplyDisposition::Replay
    );
    assert_eq!(restored.revision(), revision);
    assert_eq!(
        restored.state().attention,
        AttentionState::PermissionCandidate
    );
    restored.state.attention = AttentionState::Permission;
    assert_eq!(
        restored.snapshot().unwrap_err(),
        SnapshotViolation::InvalidState
    );
}
