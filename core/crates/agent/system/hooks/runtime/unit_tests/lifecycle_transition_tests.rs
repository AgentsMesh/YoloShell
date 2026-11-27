use super::test_support::{
    CHILD, CLAUDE_VERSION, FIRST_POST_START_SEQUENCE, TOOL, claude_envelope, claude_started,
    identity,
};
use super::{AttentionState, EventKind, HookIdentity, HookProviderKind, Reducer, TurnState};

struct Scenario {
    identity: HookIdentity,
    reducer: Reducer,
    sequence: u64,
}

impl Scenario {
    fn new() -> Self {
        let identity = identity();
        let mut reducer =
            Reducer::new(identity.clone(), HookProviderKind::Claude, CLAUDE_VERSION).unwrap();
        claude_started(&mut reducer, &identity);
        Self {
            identity,
            reducer,
            sequence: FIRST_POST_START_SEQUENCE,
        }
    }

    fn apply(&mut self, event: EventKind, child: bool, tool: bool) {
        let mut event = claude_envelope(&self.identity, self.sequence, event);
        self.sequence += super::FIRST_SEQUENCE;
        event.agent_id = child.then(|| CHILD.to_owned());
        event.tool_use_id = tool.then(|| TOOL.to_owned());
        self.reducer.apply(event).unwrap();
    }

    fn scope(&self, child: bool) -> (TurnState, AttentionState, usize, bool) {
        if child {
            let state = &self.reducer.state().children[CHILD];
            (
                state.turn,
                state.attention,
                state.active_tools.len(),
                state.stop_candidate.is_some(),
            )
        } else {
            let state = self.reducer.state();
            (
                state.turn,
                state.attention,
                state.active_tools.len(),
                state.stop_candidate.is_some(),
            )
        }
    }
}

#[test]
fn execution_transitions_consume_permission_attention_in_every_scope() {
    for child in [false, true] {
        let mut scenario = Scenario::new();
        if child {
            scenario.apply(EventKind::SubagentStart, true, false);
        }
        scenario.apply(EventKind::PermissionRequest, child, false);
        assert_eq!(scenario.scope(child).1, AttentionState::PermissionCandidate);
        scenario.apply(EventKind::PreToolUse, child, true);
        assert_eq!(scenario.scope(child).1, AttentionState::None);
        scenario.apply(EventKind::PermissionRequest, child, false);
        scenario.apply(EventKind::PostToolUse, child, true);
        assert_eq!(
            scenario.scope(child),
            (TurnState::Running, AttentionState::None, 0, false)
        );
    }
}

#[test]
fn recoverable_failures_do_not_fail_the_turn() {
    for child in [false, true] {
        let mut scenario = Scenario::new();
        if child {
            scenario.apply(EventKind::SubagentStart, true, false);
        }
        scenario.apply(EventKind::PreToolUse, child, true);
        scenario.apply(EventKind::PostToolUseFailure, child, true);
        assert_eq!(
            scenario.scope(child),
            (TurnState::Running, AttentionState::None, 0, false)
        );
        scenario.apply(EventKind::PermissionRequest, child, false);
        scenario.apply(EventKind::PermissionDenied, child, false);
        assert_eq!(
            scenario.scope(child),
            (TurnState::Running, AttentionState::None, 0, false)
        );
    }
}

#[test]
fn terminal_events_clear_transient_scope_state() {
    for child in [false, true] {
        let mut stopped = Scenario::new();
        if child {
            stopped.apply(EventKind::SubagentStart, true, false);
        }
        stopped.apply(EventKind::PreToolUse, child, true);
        stopped.apply(EventKind::PermissionRequest, child, false);
        stopped.apply(EventKind::Stop, child, false);
        assert_eq!(
            stopped.scope(child),
            (TurnState::StopCandidate, AttentionState::None, 0, true)
        );

        let mut failed = Scenario::new();
        if child {
            failed.apply(EventKind::SubagentStart, true, false);
        }
        failed.apply(EventKind::PreToolUse, child, true);
        failed.apply(EventKind::PermissionRequest, child, false);
        failed.apply(EventKind::StopFailure, child, false);
        assert_eq!(
            failed.scope(child),
            (TurnState::Failed, AttentionState::None, 0, false)
        );
    }
}
