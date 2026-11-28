use super::constants::test_values::NEXT_PROVIDER_SESSION;
use super::test_support::{
    CHILD, CHILD_TOOL, CLAUDE_VERSION, CODEX_VERSION, FIRST_POST_START_SEQUENCE, TOOL, TURN,
    claude_envelope, claude_started, envelope, identity, started,
};
use super::{
    ApplyDisposition, AttentionState, EventKind, InstrumentationState, ProcessExitDisposition,
    HookProviderKind, Reducer, SessionEndReason, SessionStartSource, TerminationCandidate, TurnState,
};

#[test]
fn session_end_boundaries_are_not_clean_exit_proof() {
    for reason in [SessionEndReason::Clear, SessionEndReason::Resume] {
        let expected = identity();
        let mut reducer =
            Reducer::new(expected.clone(), HookProviderKind::Claude, CLAUDE_VERSION).unwrap();
        claude_started(&mut reducer, &expected);
        let boundary = claude_envelope(
            &expected,
            2,
            EventKind::SessionEnd {
                reason: reason.clone(),
            },
        );
        assert_eq!(reducer.apply(boundary).unwrap(), ApplyDisposition::Applied);
        let candidate = match reason {
            SessionEndReason::Clear => TerminationCandidate::Clear,
            SessionEndReason::Resume => TerminationCandidate::Resume,
            SessionEndReason::Other => unreachable!(),
        };
        assert_eq!(reducer.state().termination_candidate, Some(candidate));
        assert!(!reducer.state().final_hook_seen);
        assert_eq!(reducer.process_exit(), ProcessExitDisposition::Unconfirmed);
    }
}

#[test]
fn a_new_session_start_clears_previous_termination_candidate() {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Claude, CLAUDE_VERSION).unwrap();
    claude_started(&mut reducer, &expected);
    reducer
        .apply(claude_envelope(
            &expected,
            2,
            EventKind::SessionEnd {
                reason: SessionEndReason::Clear,
            },
        ))
        .unwrap();
    let mut start = claude_envelope(
        &expected,
        3,
        EventKind::SessionStart {
            source: SessionStartSource::Clear,
        },
    );
    start.provider_session_id = NEXT_PROVIDER_SESSION.to_owned();
    assert_eq!(reducer.apply(start).unwrap(), ApplyDisposition::Applied);
    assert_eq!(
        reducer.state().provider_session_id.as_deref(),
        Some(NEXT_PROVIDER_SESSION)
    );
    assert_eq!(reducer.state().termination_candidate, None);
    assert_eq!(reducer.state().turn, TurnState::Idle);
}

#[test]
fn logical_boundary_clears_all_previous_turn_state() {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Claude, CLAUDE_VERSION).unwrap();
    claude_started(&mut reducer, &expected);
    let mut prompt = claude_envelope(
        &expected,
        FIRST_POST_START_SEQUENCE,
        EventKind::UserPromptSubmit,
    );
    prompt.turn_id = Some(TURN.to_owned());
    reducer.apply(prompt).unwrap();
    let mut permission = claude_envelope(
        &expected,
        FIRST_POST_START_SEQUENCE + 1,
        EventKind::PermissionRequest,
    );
    permission.turn_id = Some(TURN.to_owned());
    reducer.apply(permission).unwrap();
    let mut tool = claude_envelope(
        &expected,
        FIRST_POST_START_SEQUENCE + 2,
        EventKind::PreToolUse,
    );
    tool.tool_use_id = Some(TOOL.to_owned());
    reducer.apply(tool).unwrap();
    let mut child_start = claude_envelope(
        &expected,
        FIRST_POST_START_SEQUENCE + 3,
        EventKind::SubagentStart,
    );
    child_start.agent_id = Some(CHILD.to_owned());
    reducer.apply(child_start).unwrap();
    let mut child_tool = claude_envelope(
        &expected,
        FIRST_POST_START_SEQUENCE + 4,
        EventKind::PreToolUse,
    );
    child_tool.agent_id = Some(CHILD.to_owned());
    child_tool.tool_use_id = Some(CHILD_TOOL.to_owned());
    reducer.apply(child_tool).unwrap();
    let mut stop = claude_envelope(&expected, FIRST_POST_START_SEQUENCE + 5, EventKind::Stop);
    stop.turn_id = Some(TURN.to_owned());
    reducer.apply(stop).unwrap();

    let end = claude_envelope(
        &expected,
        FIRST_POST_START_SEQUENCE + 6,
        EventKind::SessionEnd {
            reason: SessionEndReason::Clear,
        },
    );
    reducer.apply(end).unwrap();
    let mut start = claude_envelope(
        &expected,
        FIRST_POST_START_SEQUENCE + 7,
        EventKind::SessionStart {
            source: SessionStartSource::Clear,
        },
    );
    start.provider_session_id = NEXT_PROVIDER_SESSION.to_owned();
    reducer.apply(start).unwrap();
    let state = reducer.state();
    assert_eq!(state.attention, AttentionState::None);
    assert!(state.active_tools.is_empty());
    assert!(state.children.is_empty());
    assert!(state.child_compactions.is_empty());
    assert!(state.stop_candidate.is_none());
    assert_eq!(state.instrumentation, InstrumentationState::Authoritative);
}

#[test]
fn compact_start_rejects_subagent_identity() {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    started(&mut reducer, &expected);
    let mut compact = envelope(
        &expected,
        CODEX_VERSION,
        FIRST_POST_START_SEQUENCE,
        EventKind::SessionStart {
            source: SessionStartSource::Compact,
        },
    );
    compact.agent_id = Some(CHILD.to_owned());
    assert_eq!(
        reducer.apply(compact),
        Err(super::HookViolation::InvalidTopology)
    );
    assert_eq!(
        reducer.state().instrumentation,
        InstrumentationState::Unavailable
    );
}
