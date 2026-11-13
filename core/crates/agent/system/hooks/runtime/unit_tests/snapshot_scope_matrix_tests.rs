use super::*;
use crate::test_support::{
    CHILD, CLAUDE_VERSION, TOOL, TURN, claude_envelope, claude_started, identity,
};
use crate::{ChildLifecycle, HookProviderKind, Reducer};
use uuid::Uuid;

const STEP: u64 = 1;

fn state() -> NormalizedLifecycle {
    let identity = identity();
    let mut reducer = Reducer::new(identity.clone(), HookProviderKind::Claude, CLAUDE_VERSION).unwrap();
    claude_started(&mut reducer, &identity);
    reducer.state().clone()
}

fn child() -> ChildLifecycle {
    ChildLifecycle {
        active: true,
        turn: TurnState::Running,
        attention: AttentionState::None,
        active_tools: Default::default(),
        stop_candidate: None,
    }
}

#[test]
fn stop_and_compaction_fields_require_nonempty_identifiers_and_valid_turn_anchors() {
    let valid = StopCandidate {
        event_id: Uuid::new_v4(),
        turn_id: Some(TURN.into()),
        continued: false,
    };
    assert!(valid_stop(Some(&valid)));
    let mut invalid = valid.clone();
    invalid.event_id = Uuid::nil();
    assert!(!valid_stop(Some(&invalid)));
    let mut invalid = valid.clone();
    invalid.turn_id = Some(String::new());
    assert!(!valid_stop(Some(&invalid)));
    for (prior, sequence, last, idle, accepted) in [
        (
            TurnState::Running,
            EventSequence::ZERO,
            EventSequence::FIRST,
            true,
            false,
        ),
        (
            TurnState::Running,
            EventSequence::FIRST,
            EventSequence::ZERO,
            true,
            false,
        ),
        (
            TurnState::Unknown,
            EventSequence::FIRST,
            EventSequence::FIRST,
            true,
            false,
        ),
        (
            TurnState::Compacting,
            EventSequence::FIRST,
            EventSequence::FIRST,
            true,
            false,
        ),
        (
            TurnState::Idle,
            EventSequence::FIRST,
            EventSequence::FIRST,
            false,
            false,
        ),
        (
            TurnState::Idle,
            EventSequence::FIRST,
            EventSequence::FIRST,
            true,
            true,
        ),
        (
            TurnState::StopCandidate,
            EventSequence::FIRST,
            EventSequence::FIRST,
            false,
            true,
        ),
    ] {
        assert_eq!(
            valid_phase(Some(CompactionPhase::begin(prior, sequence)), last, idle),
            accepted,
            "{prior:?} {sequence:?} {last:?}"
        );
    }
}

#[test]
fn every_active_and_inactive_child_constraint_is_checked_independently() {
    let mut original = state();
    original.children.insert(CHILD.into(), child());
    validate_scopes(&original).unwrap();
    let mutations: &[(&str, fn(&mut ChildLifecycle))] = &[
        (stringify!(active_tools), |value| {
            value.active_tools.insert(String::new());
        }),
        (stringify!(unknown_turn), |value| {
            value.turn = TurnState::Unknown
        }),
        (stringify!(idle_turn), |value| value.turn = TurnState::Idle),
        (stringify!(missing_compaction), |value| {
            value.turn = TurnState::Compacting
        }),
        (stringify!(missing_stop), |value| {
            value.turn = TurnState::StopCandidate
        }),
    ];
    for (field, mutate) in mutations {
        let mut changed = original.clone();
        mutate(changed.children.get_mut(CHILD).unwrap());
        assert_eq!(
            validate_scopes(&changed),
            Err(SnapshotViolation::InvalidState),
            "{field}"
        );
    }
    let inactive = original.children.get_mut(CHILD).unwrap();
    inactive.active = false;
    inactive.turn = TurnState::Idle;
    validate_scopes(&original).unwrap();
    for mutate in [
        |value: &mut ChildLifecycle| value.turn = TurnState::Running,
        |value: &mut ChildLifecycle| value.attention = AttentionState::Permission,
        |value: &mut ChildLifecycle| {
            value.active_tools.insert(TOOL.into());
        },
        |value: &mut ChildLifecycle| {
            value.stop_candidate = Some(StopCandidate {
                event_id: Uuid::new_v4(),
                turn_id: None,
                continued: false,
            })
        },
    ] {
        let mut changed = original.clone();
        mutate(changed.children.get_mut(CHILD).unwrap());
        assert_eq!(
            validate_scopes(&changed),
            Err(SnapshotViolation::InvalidState)
        );
    }
    original.children = [(String::new(), child())].into();
    assert_eq!(
        validate_scopes(&original),
        Err(SnapshotViolation::InvalidState)
    );
}

#[path = "snapshot_anchor_matrix_tests.rs"]
mod snapshot_anchor_matrix_tests;
