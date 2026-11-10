use super::*;
use crate::claude_hooks::{ClaudeHookOperation, INITIAL_CLAUDE_EVENT_SEQUENCE};

pub(super) fn corrupt_receipt_references(fixture: &Fixture, authenticated: bool) {
    let original = fixture.snapshot();
    let queue = &original.invocations[FIRST_QUEUE];
    let ClaudeHookOperation::Lifecycle {
        event_sequence,
        event: ClaudeLifecycleEvent::StateChanged { event_id, .. },
    } = &queue.pending[STATE_CHANGED_INDEX].request.operation
    else {
        unreachable!()
    };
    let cases = [
        (
            stringify!(nil_receipt_event),
            AuthenticatedSessionStart {
                event_id: Uuid::nil(),
                ..fixture.accepted
            },
        ),
        (
            stringify!(receipt_before_initial_sequence),
            AuthenticatedSessionStart {
                event_sequence: INITIAL_CLAUDE_EVENT_SEQUENCE - STEP,
                ..fixture.accepted
            },
        ),
        (
            stringify!(receipt_at_unreserved_next_sequence),
            AuthenticatedSessionStart {
                event_sequence: queue.next_sequence,
                ..fixture.accepted
            },
        ),
        (
            stringify!(receipt_wrong_event_identity),
            AuthenticatedSessionStart {
                event_id: Uuid::new_v4(),
                ..fixture.accepted
            },
        ),
        (
            stringify!(receipt_names_non_start_event),
            AuthenticatedSessionStart {
                event_sequence: *event_sequence,
                event_id: *event_id,
            },
        ),
    ];
    for (label, accepted) in cases {
        let mut damaged = original.clone();
        let queue = &mut damaged.invocations[FIRST_QUEUE];
        if authenticated {
            queue.authenticated_session_start = Some(accepted);
        } else {
            queue.prepared_session_start.as_mut().unwrap().accepted = accepted;
        }
        fixture.reject_corruption(label, &damaged);
    }
}
