use super::test_support::*;
use super::*;
use serde_json::{from_value, to_value};

#[test]
fn sequence_and_replay_are_strict() {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    let first = envelope(
        &expected,
        CODEX_VERSION,
        2,
        EventKind::SessionStart {
            source: SessionStartSource::Startup,
        },
    );
    assert_eq!(reducer.apply(first), Err(HookViolation::SequenceGap));
    assert_eq!(reducer.state().process, ProcessState::Unknown);

    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    started(&mut reducer, &expected);
    let prompt = envelope(&expected, CODEX_VERSION, 2, EventKind::UserPromptSubmit);
    assert_eq!(
        reducer.apply(prompt.clone()).unwrap(),
        ApplyDisposition::Applied
    );
    assert_eq!(reducer.apply(prompt).unwrap(), ApplyDisposition::Replay);
    let conflict = envelope(&expected, CODEX_VERSION, 2, EventKind::Stop);
    assert_eq!(
        reducer.apply(conflict),
        Err(HookViolation::DuplicateConflict)
    );
}

#[test]
fn identity_mismatch_revokes_future_events() {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    let mut bad = envelope(
        &expected,
        CODEX_VERSION,
        1,
        EventKind::SessionStart {
            source: SessionStartSource::Startup,
        },
    );
    bad.identity.generation = Generation(Generation::INITIAL.0 + FIRST_SEQUENCE);
    assert_eq!(reducer.apply(bad), Err(HookViolation::GenerationMismatch));
    assert_eq!(
        reducer.apply(envelope(&expected, CODEX_VERSION, 1, EventKind::Stop)),
        Err(HookViolation::Revoked)
    );
    assert_eq!(
        reducer.state().instrumentation,
        InstrumentationState::Unavailable
    );
}

#[test]
fn stop_is_candidate_and_children_are_isolated() {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    started(&mut reducer, &expected);
    let mut prompt = envelope(&expected, CODEX_VERSION, 2, EventKind::UserPromptSubmit);
    prompt.turn_id = Some(TURN.to_owned());
    reducer.apply(prompt).unwrap();
    let mut stop = envelope(&expected, CODEX_VERSION, 3, EventKind::Stop);
    stop.turn_id = Some(TURN.to_owned());
    stop.stop_hook_active = Some(true);
    reducer.apply(stop).unwrap();
    assert_eq!(reducer.state().turn, TurnState::StopCandidate);
    assert!(reducer.state().stop_candidate.is_some());
    let mut child_start = envelope(&expected, CODEX_VERSION, 4, EventKind::SubagentStart);
    child_start.agent_id = Some(CHILD.to_owned());
    reducer.apply(child_start).unwrap();
    let mut child_stop = envelope(&expected, CODEX_VERSION, 5, EventKind::SubagentStop);
    child_stop.agent_id = Some(CHILD.to_owned());
    reducer.apply(child_stop).unwrap();
    assert_eq!(reducer.state().turn, TurnState::StopCandidate);
    assert!(!reducer.state().children[CHILD].active);
}

#[test]
fn compaction_restores_turn_and_missing_final_is_unknown() {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    started(&mut reducer, &expected);
    reducer
        .apply(envelope(
            &expected,
            CODEX_VERSION,
            2,
            EventKind::UserPromptSubmit,
        ))
        .unwrap();
    reducer
        .apply(envelope(&expected, CODEX_VERSION, 3, EventKind::PreCompact))
        .unwrap();
    assert_eq!(reducer.state().turn, TurnState::Compacting);
    reducer
        .apply(envelope(
            &expected,
            CODEX_VERSION,
            4,
            EventKind::PostCompact,
        ))
        .unwrap();
    assert_eq!(reducer.state().turn, TurnState::Running);
    assert_eq!(reducer.process_exit(), ProcessExitDisposition::Unconfirmed);
    assert_eq!(reducer.state().turn, TurnState::Unknown);
    assert_eq!(
        reducer.state().instrumentation,
        InstrumentationState::Unavailable
    );
}

#[test]
fn final_hook_exit_and_event_serialization() {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    started(&mut reducer, &expected);
    reducer
        .apply(envelope(
            &expected,
            CODEX_VERSION,
            2,
            EventKind::SessionEnd {
                reason: SessionEndReason::Other,
            },
        ))
        .unwrap();
    assert_eq!(reducer.process_exit(), ProcessExitDisposition::Confirmed);
    let encoded = to_value(EventKind::UserPromptSubmit).unwrap();
    assert_eq!(
        from_value::<EventKind>(encoded).unwrap(),
        EventKind::UserPromptSubmit
    );
}

#[test]
fn duplicate_event_id_conflict_is_fail_closed() {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    started(&mut reducer, &expected);
    let mut second = envelope(&expected, CODEX_VERSION, 2, EventKind::UserPromptSubmit);
    reducer.apply(second.clone()).unwrap();
    second.sequence = EventSequence(3);
    assert_eq!(reducer.apply(second), Err(HookViolation::DuplicateConflict));
    assert_eq!(reducer.state().process, ProcessState::Unknown);
}

#[test]
fn process_exit_identity_is_fenced() {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    let wrong = ProcessIdentity::new(expected.process.pid, expected.process.start_time + 1);
    assert_eq!(
        reducer.process_exit_with_identity(wrong),
        Err(HookViolation::ProcessMismatch)
    );
    assert_eq!(reducer.process_exit(), ProcessExitDisposition::Revoked);
}

#[test]
fn provider_mapper_exposes_claude_extensions() {
    assert_eq!(
        ClaudeEventMapper.map(event_names::POST_TOOL_USE_FAILURE),
        Some(EventKind::PostToolUseFailure)
    );
    assert_eq!(HookProviderKind::Codex.as_str(), provider_names::CODEX);
}
