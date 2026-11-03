use super::*;

#[test]
fn stop_anchors_cannot_be_rebound_to_another_child_event_turn_or_continuation() {
    let mut original = state();
    let sequence = EventSequence(original.last_sequence.0 + STEP);
    let mut event = claude_envelope(&original.expected_identity, sequence.0, EventKind::Stop);
    event.turn_id = Some(TURN.into());
    original.stop_candidate = Some(StopCandidate {
        event_id: event.event_id,
        turn_id: event.turn_id.clone(),
        continued: false,
    });
    original.event_ids.insert(event.event_id, sequence);
    original.history.insert(sequence, event);
    original.last_sequence = sequence;
    validate_scopes(&original).unwrap();
    let mutations: &[fn(&mut crate::HookEnvelope)] = &[
        |value| value.agent_id = Some(CHILD.into()),
        |value| value.event = EventKind::UserPromptSubmit,
        |value| value.turn_id = None,
        |value| value.stop_hook_active = Some(true),
    ];
    for mutate in mutations {
        let mut changed = original.clone();
        mutate(changed.history.get_mut(&sequence).unwrap());
        assert_eq!(
            validate_scopes(&changed),
            Err(SnapshotViolation::InvalidState)
        );
    }
    original.history.remove(&sequence);
    assert_eq!(
        validate_scopes(&original),
        Err(SnapshotViolation::InvalidState)
    );
}

#[test]
fn compaction_anchors_cannot_reference_foreign_events_or_orphaned_children() {
    let mut original = state();
    let phase = CompactionPhase::begin(TurnState::Running, original.last_sequence);
    original.compaction = Some(phase);
    original
        .history
        .get_mut(&original.last_sequence)
        .unwrap()
        .event = EventKind::PreCompact;
    validate_scopes(&original).unwrap();
    for mutate in [
        |value: &mut crate::HookEnvelope| value.agent_id = Some(CHILD.into()),
        |value: &mut crate::HookEnvelope| value.event = EventKind::Stop,
    ] {
        let mut changed = original.clone();
        mutate(changed.history.get_mut(&changed.last_sequence).unwrap());
        assert_eq!(
            validate_scopes(&changed),
            Err(SnapshotViolation::InvalidState)
        );
    }
    original.compaction = None;
    original.child_compactions.insert(CHILD.into(), phase);
    assert_eq!(
        validate_scopes(&original),
        Err(SnapshotViolation::InvalidState)
    );
}
