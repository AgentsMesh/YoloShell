use super::*;
#[path = "../test_support/snapshot_validation_matrix.rs"]
mod fixture;
use crate::test_support::{CLAUDE_VERSION, SESSION, claude_envelope};
use fixture::*;

#[test]
fn pending_snapshot_rejects_each_piece_of_premature_lifecycle_state() {
    rejects_each(
        &initial(),
        validate_initial,
        &[
            ("provider_session", |value| {
                value.provider_session_id = Some(SESSION.into())
            }),
            ("sequence", |value| {
                value.last_sequence = EventSequence::FIRST
            }),
            ("history", |value| {
                value.history.insert(
                    EventSequence::FIRST,
                    claude_envelope(
                        &value.expected_identity,
                        crate::FIRST_SEQUENCE,
                        crate::EventKind::UserPromptSubmit,
                    ),
                );
            }),
            (crate::constants::snapshot_fields::EVENT_IDS, |value| {
                value
                    .event_ids
                    .insert(uuid::Uuid::new_v4(), EventSequence::FIRST);
            }),
            ("cwd", |value| value.current_cwd = Some(CWD.into())),
            ("transcript", |value| {
                value.current_transcript_path = Some(TRANSCRIPT.into())
            }),
            ("turn", |value| value.turn = TurnState::Running),
            ("attention", |value| {
                value.attention = AttentionState::Permission
            }),
            ("tools", |value| {
                value.active_tools.insert(TOOL.into());
            }),
            ("children", |value| {
                value
                    .children
                    .insert(CHILD.into(), fixture::inactive_child());
            }),
            ("stop", |value| value.stop_candidate = Some(stop())),
            ("termination", |value| {
                value.termination_candidate = Some(TerminationCandidate::Final)
            }),
            ("compaction", |value| value.compaction = Some(phase())),
            ("child_compaction", |value| {
                value.child_compactions.insert(CHILD.into(), phase());
            }),
        ],
    );
}

#[test]
fn degraded_snapshot_cannot_retain_live_work_or_terminal_authority() {
    let original = degraded();
    rejects_each(
        &original,
        validate_degraded,
        &[
            ("provider_session", |value| value.provider_session_id = None),
            ("empty_session", |value| {
                value.provider_session_id = Some(String::new())
            }),
            ("turn", |value| value.turn = TurnState::Running),
            ("attention", |value| {
                value.attention = AttentionState::Permission
            }),
            ("tools", |value| {
                value.active_tools.insert(TOOL.into());
            }),
            ("children", |value| {
                value
                    .children
                    .insert(CHILD.into(), fixture::inactive_child());
            }),
            ("stop", |value| value.stop_candidate = Some(stop())),
            ("termination", |value| {
                value.termination_candidate = Some(TerminationCandidate::Final)
            }),
            ("final_hook", |value| value.final_hook_seen = true),
            ("compaction", |value| value.compaction = Some(phase())),
            ("child_compaction", |value| {
                value.child_compactions.insert(CHILD.into(), phase());
            }),
        ],
    );
}

#[test]
fn terminal_root_rejects_residual_turn_attention_tools_stop_and_compaction() {
    let original = terminal();
    rejects_each(
        &original,
        validate_terminal_root,
        &[
            ("turn", |value| value.turn = TurnState::Running),
            ("attention", |value| {
                value.attention = AttentionState::Permission
            }),
            ("tools", |value| {
                value.active_tools.insert(TOOL.into());
            }),
            ("stop", |value| value.stop_candidate = Some(stop())),
            ("compaction", |value| value.compaction = Some(phase())),
        ],
    );
}

#[test]
fn inactive_snapshots_reject_root_and_child_activity_independently() {
    let original = exited();
    rejects_each(
        &original,
        |value| validate_inactive(value, TurnState::Unknown),
        &[
            ("turn", |value| value.turn = TurnState::Running),
            ("attention", |value| {
                value.attention = AttentionState::UserInput
            }),
            ("tools", |value| {
                value.active_tools.insert(TOOL.into());
            }),
            ("stop", |value| value.stop_candidate = Some(stop())),
            ("compaction", |value| value.compaction = Some(phase())),
            ("child_compaction", |value| {
                value.child_compactions.insert(CHILD.into(), phase());
            }),
            ("child_active", |value| {
                value.children.get_mut(CHILD).unwrap().active = true
            }),
            ("child_turn", |value| {
                value.children.get_mut(CHILD).unwrap().turn = TurnState::Running
            }),
            ("child_attention", |value| {
                value.children.get_mut(CHILD).unwrap().attention = AttentionState::Permission
            }),
            ("child_tools", |value| {
                value
                    .children
                    .get_mut(CHILD)
                    .unwrap()
                    .active_tools
                    .insert(TOOL.into());
            }),
            ("child_stop", |value| {
                value.children.get_mut(CHILD).unwrap().stop_candidate = Some(stop())
            }),
        ],
    );
}

#[test]
fn public_snapshot_validation_rejects_individually_corrupted_initial_identity() {
    let mutations: &[Mutation] = &[
        ("expected_identity", |value| {
            value.expected_identity.invocation_id = uuid::Uuid::nil()
        }),
        ("generation", |value| {
            value.generation = crate::Generation(u64::MAX)
        }),
        ("session_sequence", |value| {
            value.provider_session_id = Some(SESSION.into())
        }),
        ("revoked_without_evidence", |value| value.revoked = true),
        ("final_without_candidate", |value| {
            value.final_hook_seen = true
        }),
    ];
    rejects_each(&initial(), validate_lifecycle, mutations);
    for (field, mutate) in mutations {
        let mut reducer = crate::Reducer::new(
            crate::test_support::identity(),
            crate::HookProviderKind::Claude,
            CLAUDE_VERSION,
        )
        .unwrap();
        mutate(&mut reducer.state);
        assert_eq!(
            reducer.snapshot().unwrap_err(),
            SnapshotViolation::InvalidState,
            "{field}"
        );
    }
}
