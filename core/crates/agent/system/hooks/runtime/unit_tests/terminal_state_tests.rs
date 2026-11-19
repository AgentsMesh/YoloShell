use super::test_support::{
    CHILD, CHILD_TOOL, CODEX_VERSION, FIRST_POST_START_SEQUENCE, TOOL, TURN, envelope, identity,
    started,
};
use super::{
    AttentionState, EventKind, HookIdentity, HookViolation, InstrumentationState,
    NormalizedLifecycle, ProcessExitDisposition, ProcessState, HookProviderKind, Reducer,
    SessionEndReason, TerminationCandidate, TurnState,
};

struct Scenario {
    identity: HookIdentity,
    reducer: Reducer,
    next_sequence: u64,
}

impl Scenario {
    fn new() -> Self {
        let identity = identity();
        let mut reducer =
            Reducer::new(identity.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
        started(&mut reducer, &identity);
        Self {
            identity,
            reducer,
            next_sequence: FIRST_POST_START_SEQUENCE,
        }
    }

    fn envelope(&mut self, event: EventKind) -> super::HookEnvelope {
        let envelope = envelope(&self.identity, CODEX_VERSION, self.next_sequence, event);
        self.next_sequence += super::FIRST_SEQUENCE;
        envelope
    }

    fn apply(&mut self, event: EventKind) {
        let envelope = self.envelope(event);
        self.reducer.apply(envelope).unwrap();
    }

    fn apply_child(&mut self, event: EventKind) {
        let mut envelope = self.envelope(event);
        envelope.agent_id = Some(CHILD.to_owned());
        self.reducer.apply(envelope).unwrap();
    }

    fn apply_tool(&mut self, agent: Option<&str>, tool: &str) {
        let mut envelope = self.envelope(EventKind::PreToolUse);
        envelope.agent_id = agent.map(str::to_owned);
        envelope.tool_use_id = Some(tool.to_owned());
        self.reducer.apply(envelope).unwrap();
    }

    fn apply_stop(&mut self, agent: Option<&str>) {
        let mut envelope = self.envelope(EventKind::Stop);
        envelope.agent_id = agent.map(str::to_owned);
        envelope.turn_id = Some(TURN.to_owned());
        self.reducer.apply(envelope).unwrap();
    }

    fn populate_active_state(&mut self) {
        self.apply(EventKind::PermissionRequest);
        self.apply_tool(None, TOOL);
        self.apply_stop(None);
        self.apply(EventKind::PreCompact);
        self.apply_child(EventKind::SubagentStart);
        self.apply_child(EventKind::PermissionRequest);
        self.apply_tool(Some(CHILD), CHILD_TOOL);
        self.apply_stop(Some(CHILD));
        self.apply_child(EventKind::PreCompact);
    }
}

#[test]
fn process_exit_canonicalizes_every_active_scope() {
    let mut scenario = Scenario::new();
    scenario.populate_active_state();
    assert_eq!(scenario.reducer.state().turn, TurnState::Compacting);
    assert!(scenario.reducer.state().compaction.is_some());
    assert!(
        scenario
            .reducer
            .state()
            .child_compactions
            .contains_key(CHILD)
    );

    assert_eq!(
        scenario.reducer.process_exit(),
        ProcessExitDisposition::Unconfirmed
    );
    let state = scenario.reducer.state();
    assert_eq!(state.process, ProcessState::Exited);
    assert_eq!(state.instrumentation, InstrumentationState::Unavailable);
    assert_inactive(state, TurnState::Unknown);
}

#[test]
fn confirmed_exit_makes_children_idle_and_inactive() {
    let mut scenario = Scenario::new();
    scenario.populate_active_state();
    scenario.apply(EventKind::SessionEnd {
        reason: SessionEndReason::Other,
    });

    assert_eq!(
        scenario.reducer.process_exit(),
        ProcessExitDisposition::Confirmed
    );
    let state = scenario.reducer.state();
    assert_eq!(
        state.termination_candidate,
        Some(TerminationCandidate::Final)
    );
    assert!(state.final_hook_seen);
    assert_inactive(state, TurnState::Idle);
}

#[test]
fn invalidation_clears_all_retained_and_active_state() {
    let mut scenario = Scenario::new();
    scenario.populate_active_state();
    let sequence_after_gap = scenario.next_sequence + super::FIRST_SEQUENCE;
    let invalid = envelope(
        &scenario.identity,
        CODEX_VERSION,
        sequence_after_gap,
        EventKind::UserPromptSubmit,
    );
    assert_eq!(
        scenario.reducer.apply(invalid),
        Err(HookViolation::SequenceGap)
    );

    let state = scenario.reducer.state();
    assert!(state.revoked);
    assert_eq!(state.process, ProcessState::Unknown);
    assert_eq!(state.instrumentation, InstrumentationState::Unavailable);
    assert_eq!(state.termination_candidate, None);
    assert!(!state.final_hook_seen);
    assert_inactive(state, TurnState::Unknown);
}

#[test]
fn turn_events_cancel_stale_compaction_state() {
    let mut scenario = Scenario::new();
    scenario.apply(EventKind::PreCompact);
    assert!(scenario.reducer.state().compaction.is_some());
    scenario.apply(EventKind::PermissionRequest);
    assert!(scenario.reducer.state().compaction.is_none());

    scenario.apply_child(EventKind::SubagentStart);
    scenario.apply_child(EventKind::PreCompact);
    assert!(
        scenario
            .reducer
            .state()
            .child_compactions
            .contains_key(CHILD)
    );
    scenario.apply_child(EventKind::PermissionRequest);
    assert!(
        !scenario
            .reducer
            .state()
            .child_compactions
            .contains_key(CHILD)
    );
    assert_eq!(
        scenario.reducer.process_exit(),
        ProcessExitDisposition::Unconfirmed
    );
}

fn assert_inactive(state: &NormalizedLifecycle, turn: TurnState) {
    assert_eq!(state.turn, turn);
    assert_eq!(state.attention, AttentionState::None);
    assert!(state.active_tools.is_empty());
    assert!(state.stop_candidate.is_none());
    assert!(state.compaction.is_none());
    assert!(state.child_compactions.is_empty());
    assert!(!state.children.is_empty());
    for child in state.children.values() {
        assert!(!child.active);
        assert_eq!(child.turn, turn);
        assert_eq!(child.attention, AttentionState::None);
        assert!(child.active_tools.is_empty());
        assert!(child.stop_candidate.is_none());
    }
}
