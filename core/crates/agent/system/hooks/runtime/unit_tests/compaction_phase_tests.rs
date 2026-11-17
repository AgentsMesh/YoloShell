use super::test_support::{
    CHILD, CLAUDE_VERSION, FIRST_POST_START_SEQUENCE, SESSION, claude_envelope, claude_started,
    identity,
};
use super::{
    ApplyDisposition, EventKind, HookEnvelope, HookIdentity, HookProviderKind, Reducer,
    SessionStartSource, TurnState,
};

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

    fn child_event(&mut self, event: EventKind) -> HookEnvelope {
        let mut envelope = self.event(event);
        envelope.agent_id = Some(CHILD.to_owned());
        envelope
    }

    fn apply(&mut self, envelope: HookEnvelope, expected: ApplyDisposition) {
        assert_eq!(self.reducer.apply(envelope).unwrap(), expected);
    }
}

#[test]
fn unmatched_completions_are_stale_without_turn_mutation() {
    let mut fixture = Fixture::new();
    let post = fixture.event(EventKind::PostCompact);
    fixture.apply(post, ApplyDisposition::Stale);
    assert_eq!(fixture.reducer.state().turn, TurnState::Idle);

    let mut start = fixture.event(EventKind::SessionStart {
        source: SessionStartSource::Compact,
    });
    start.provider_session_id = SESSION.to_owned();
    fixture.apply(start, ApplyDisposition::Stale);
    assert_eq!(fixture.reducer.state().turn, TurnState::Idle);
    assert!(fixture.reducer.state().compaction.is_none());
}

#[test]
fn compact_session_start_completes_the_root_epoch() {
    let mut fixture = Fixture::new();
    let prompt = fixture.event(EventKind::UserPromptSubmit);
    fixture.apply(prompt, ApplyDisposition::Applied);
    let pre = fixture.event(EventKind::PreCompact);
    fixture.apply(pre, ApplyDisposition::Applied);
    assert_eq!(fixture.reducer.state().turn, TurnState::Compacting);

    let start = fixture.event(EventKind::SessionStart {
        source: SessionStartSource::Compact,
    });
    fixture.apply(start, ApplyDisposition::Applied);
    assert_eq!(fixture.reducer.state().turn, TurnState::Running);
    assert!(fixture.reducer.state().compaction.is_none());
    assert_eq!(
        fixture.reducer.state().provider_session_id.as_deref(),
        Some(SESSION)
    );
}

#[test]
fn nested_precompact_cannot_replace_the_active_epoch() {
    let mut fixture = Fixture::new();
    let first = fixture.event(EventKind::PreCompact);
    fixture.apply(first.clone(), ApplyDisposition::Applied);
    let phase = fixture.reducer.state().compaction;
    let nested = fixture.event(EventKind::PreCompact);
    fixture.apply(nested, ApplyDisposition::Stale);
    assert_eq!(fixture.reducer.state().compaction, phase);
    assert_eq!(fixture.reducer.state().turn, TurnState::Compacting);

    let post = fixture.event(EventKind::PostCompact);
    fixture.apply(post, ApplyDisposition::Applied);
    assert_eq!(fixture.reducer.state().turn, TurnState::Idle);
    assert!(fixture.reducer.state().compaction.is_none());
    fixture.apply(first, ApplyDisposition::Replay);
    assert!(fixture.reducer.state().compaction.is_none());
}

#[test]
fn delayed_completion_cannot_overwrite_an_intervening_turn() {
    let mut fixture = Fixture::new();
    let pre = fixture.event(EventKind::PreCompact);
    fixture.apply(pre, ApplyDisposition::Applied);
    let denied = fixture.event(EventKind::PermissionDenied);
    fixture.apply(denied, ApplyDisposition::Applied);
    assert_eq!(fixture.reducer.state().turn, TurnState::Running);
    assert!(fixture.reducer.state().compaction.is_none());

    let delayed = fixture.event(EventKind::PostCompact);
    fixture.apply(delayed, ApplyDisposition::Stale);
    assert_eq!(fixture.reducer.state().turn, TurnState::Running);
}

#[test]
fn root_and_child_compaction_epochs_do_not_cross_complete() {
    let mut fixture = Fixture::new();
    let child_start = fixture.child_event(EventKind::SubagentStart);
    fixture.apply(child_start, ApplyDisposition::Applied);
    let root_pre = fixture.event(EventKind::PreCompact);
    fixture.apply(root_pre, ApplyDisposition::Applied);
    let unmatched_child_post = fixture.child_event(EventKind::PostCompact);
    fixture.apply(unmatched_child_post, ApplyDisposition::Stale);
    assert!(fixture.reducer.state().compaction.is_some());

    let child_pre = fixture.child_event(EventKind::PreCompact);
    fixture.apply(child_pre, ApplyDisposition::Applied);
    let root_post = fixture.event(EventKind::PostCompact);
    fixture.apply(root_post, ApplyDisposition::Applied);
    assert!(fixture.reducer.state().compaction.is_none());
    assert!(
        fixture
            .reducer
            .state()
            .child_compactions
            .contains_key(CHILD)
    );

    let child_post = fixture.child_event(EventKind::PostCompact);
    fixture.apply(child_post, ApplyDisposition::Applied);
    assert!(fixture.reducer.state().child_compactions.is_empty());
    assert_eq!(
        fixture.reducer.state().children.get(CHILD).unwrap().turn,
        TurnState::Running
    );
}
