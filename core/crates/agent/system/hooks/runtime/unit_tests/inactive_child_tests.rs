use super::test_support::{
    CHILD, CLAUDE_VERSION, FIRST_POST_START_SEQUENCE, assert_stale_bookkeeping, claude_envelope,
    claude_started, identity,
};
use super::{ApplyDisposition, EventKind, HookEnvelope, NotificationKind, HookProviderKind, Reducer};

#[test]
fn stopped_child_events_are_stale_with_exact_bookkeeping_and_replay() {
    let identity = identity();
    let mut reducer = Reducer::new(identity.clone(), HookProviderKind::Claude, CLAUDE_VERSION).unwrap();
    claude_started(&mut reducer, &identity);
    let mut sequence = FIRST_POST_START_SEQUENCE;
    apply_child_lifecycle(
        &mut reducer,
        &identity,
        &mut sequence,
        EventKind::SubagentStart,
    );
    apply_child_lifecycle(
        &mut reducer,
        &identity,
        &mut sequence,
        EventKind::SubagentStop,
    );
    assert!(!reducer.state().children[CHILD].active);

    for event_kind in ordinary_child_events() {
        let event = child_event(&identity, sequence, event_kind);
        sequence += super::FIRST_SEQUENCE;
        let before = reducer.state().clone();
        assert_eq!(
            reducer.apply(event.clone()).unwrap(),
            ApplyDisposition::Stale
        );
        assert_stale_bookkeeping(&before, reducer.state(), &event);
        let after_stale = reducer.state().clone();
        assert_eq!(reducer.apply(event).unwrap(), ApplyDisposition::Replay);
        assert_eq!(reducer.state(), &after_stale);
    }
}

fn apply_child_lifecycle(
    reducer: &mut Reducer,
    identity: &super::HookIdentity,
    sequence: &mut u64,
    event: EventKind,
) {
    let envelope = child_event(identity, *sequence, event);
    *sequence += super::FIRST_SEQUENCE;
    assert_eq!(reducer.apply(envelope).unwrap(), ApplyDisposition::Applied);
}

fn child_event(identity: &super::HookIdentity, sequence: u64, event: EventKind) -> HookEnvelope {
    let mut envelope = claude_envelope(identity, sequence, event);
    envelope.agent_id = Some(CHILD.to_owned());
    envelope
}

fn ordinary_child_events() -> impl IntoIterator<Item = EventKind> {
    [
        EventKind::UserPromptSubmit,
        EventKind::PreToolUse,
        EventKind::PermissionRequest,
        EventKind::PostToolUse,
        EventKind::PostToolUseFailure,
        EventKind::PreCompact,
        EventKind::PostCompact,
        EventKind::Stop,
        EventKind::StopFailure,
        EventKind::Notification {
            kind: NotificationKind::PermissionPrompt,
        },
        EventKind::PermissionDenied,
        EventKind::CwdChanged,
        EventKind::Elicitation,
        EventKind::ElicitationResult,
    ]
}
