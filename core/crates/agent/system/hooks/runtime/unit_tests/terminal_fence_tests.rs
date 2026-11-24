use super::constants::test_values::NEXT_PROVIDER_SESSION;
use super::test_support::{
    CHILD, CHILD_TOOL, CLAUDE_VERSION, FIRST_POST_START_SEQUENCE, assert_stale_bookkeeping,
    claude_envelope, claude_started, identity,
};
use super::{
    ApplyDisposition, EventKind, HookEnvelope, HookIdentity, HookViolation, ProcessExitDisposition,
    HookProviderKind, Reducer, SessionEndReason, SessionStartSource, TerminationCandidate,
};

const LATE_CHILD: &str = "late-child";

struct Fixture {
    identity: HookIdentity,
    reducer: Reducer,
    next_sequence: u64,
}

impl Fixture {
    fn new() -> Self {
        let identity = identity();
        let mut reducer =
            Reducer::new(identity.clone(), HookProviderKind::Claude, CLAUDE_VERSION).unwrap();
        claude_started(&mut reducer, &identity);
        Self {
            identity,
            reducer,
            next_sequence: FIRST_POST_START_SEQUENCE,
        }
    }

    fn event(&mut self, event: EventKind) -> HookEnvelope {
        let envelope = claude_envelope(&self.identity, self.next_sequence, event);
        self.next_sequence += super::FIRST_SEQUENCE;
        envelope
    }

    fn apply(&mut self, event: EventKind) -> HookEnvelope {
        let envelope = self.event(event);
        assert_eq!(
            self.reducer.apply(envelope.clone()).unwrap(),
            ApplyDisposition::Applied
        );
        envelope
    }
}

#[test]
fn late_same_session_events_are_stale_and_preserve_final_evidence() {
    let mut fixture = Fixture::new();
    let mut child = fixture.event(EventKind::SubagentStart);
    child.agent_id = Some(CHILD.to_owned());
    assert_eq!(
        fixture.reducer.apply(child).unwrap(),
        ApplyDisposition::Applied
    );
    let end = fixture.apply(EventKind::SessionEnd {
        reason: SessionEndReason::Other,
    });

    let user_prompt = fixture.event(EventKind::UserPromptSubmit);
    let cwd = fixture.event(EventKind::CwdChanged);
    let mut child_tool = fixture.event(EventKind::PreToolUse);
    child_tool.agent_id = Some(CHILD.to_owned());
    child_tool.tool_use_id = Some(CHILD_TOOL.to_owned());
    let mut child_stop = fixture.event(EventKind::SubagentStop);
    child_stop.agent_id = Some(CHILD.to_owned());
    let mut late_child_start = fixture.event(EventKind::SubagentStart);
    late_child_start.agent_id = Some(LATE_CHILD.to_owned());
    let mut late_child_tool = fixture.event(EventKind::PreToolUse);
    late_child_tool.agent_id = Some(LATE_CHILD.to_owned());
    late_child_tool.tool_use_id = Some(CHILD_TOOL.to_owned());
    let repeated_end = fixture.event(EventKind::SessionEnd {
        reason: SessionEndReason::Other,
    });
    let late = [
        user_prompt,
        cwd,
        child_tool,
        child_stop,
        late_child_start,
        late_child_tool,
        repeated_end,
    ];
    for event in late {
        let before = fixture.reducer.state().clone();
        assert_eq!(
            fixture.reducer.apply(event.clone()).unwrap(),
            ApplyDisposition::Stale
        );
        assert_stale_bookkeeping(&before, fixture.reducer.state(), &event);
        let after_stale = fixture.reducer.state().clone();
        assert_eq!(
            fixture.reducer.apply(event).unwrap(),
            ApplyDisposition::Replay
        );
        assert_eq!(fixture.reducer.state(), &after_stale);
    }
    assert!(!fixture.reducer.state().children.contains_key(LATE_CHILD));
    let before_replay = fixture.reducer.state().clone();
    assert_eq!(
        fixture.reducer.apply(end).unwrap(),
        ApplyDisposition::Replay
    );
    assert_eq!(fixture.reducer.state(), &before_replay);
    assert_eq!(
        fixture.reducer.process_exit(),
        ProcessExitDisposition::Confirmed
    );
}

#[test]
fn matching_recovery_start_supersedes_terminal_fence() {
    let cases = [
        (
            SessionEndReason::Clear,
            TerminationCandidate::Clear,
            SessionStartSource::Clear,
        ),
        (
            SessionEndReason::Resume,
            TerminationCandidate::Resume,
            SessionStartSource::Resume,
        ),
    ];
    for (reason, candidate, source) in cases {
        let mut fixture = Fixture::new();
        fixture.apply(EventKind::SessionEnd { reason });
        assert_eq!(
            fixture.reducer.state().termination_candidate,
            Some(candidate)
        );
        let late = fixture.event(EventKind::CwdChanged);
        assert_eq!(
            fixture.reducer.apply(late).unwrap(),
            ApplyDisposition::Stale
        );
        let mut start = fixture.event(EventKind::SessionStart { source });
        start.provider_session_id = NEXT_PROVIDER_SESSION.to_owned();
        assert_eq!(
            fixture.reducer.apply(start).unwrap(),
            ApplyDisposition::Applied
        );
        assert_eq!(fixture.reducer.state().termination_candidate, None);
        assert_eq!(
            fixture.reducer.state().provider_session_id.as_deref(),
            Some(NEXT_PROVIDER_SESSION)
        );
    }
}

#[test]
fn mismatched_recovery_start_revokes_terminal_authority() {
    for (reason, source) in [
        (SessionEndReason::Clear, SessionStartSource::Resume),
        (SessionEndReason::Resume, SessionStartSource::Clear),
    ] {
        let mut fixture = Fixture::new();
        fixture.apply(EventKind::SessionEnd { reason });
        let start = fixture.event(EventKind::SessionStart { source });
        assert_eq!(
            fixture.reducer.apply(start),
            Err(HookViolation::InvalidTopology)
        );
        assert_eq!(
            fixture.reducer.process_exit(),
            ProcessExitDisposition::Revoked
        );
    }
}
