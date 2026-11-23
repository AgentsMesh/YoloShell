use crate::{fixtures::*, support::*};
use yolo_hub_delegation::FIRST_POLICY_EPOCH;
use yolo_hub_peer_protocol::*;
use yolo_hub_rpc_protocol::{Command, Reply, Request, Response};
use yolo_hub_terminal_protocol::{AttachRequest, OutputCursor, TerminalEvent};

#[test]
fn call_is_bound_to_origin_recipient_clock_and_each_authenticated_hop() {
    let fixture = Fixture::new();
    let call = fixture.call();
    let (origin, relay, entry) = (
        fixture.origin.identity,
        fixture.relay.identity,
        fixture.entry.identity,
    );
    call.admit(relay.hub(), entry, origin, FIRST_POLICY_EPOCH, NOW)
        .unwrap();
    call.admit(origin.hub(), relay, origin, FIRST_POLICY_EPOCH, NOW)
        .unwrap();
    assert_eq!(call.next_hop(entry.hub()), Some(relay.hub()));
    assert_eq!(call.next_hop(relay.hub()), Some(origin.hub()));
    assert_eq!(call.next_hop(origin.hub()), None);
    assert!(
        call.admit(origin.hub(), entry, origin, FIRST_POLICY_EPOCH, NOW)
            .is_err()
    );
    assert!(
        call.admit(origin.hub(), relay, origin, FIRST_POLICY_EPOCH + STEP, NOW)
            .is_err()
    );
    assert!(
        call.admit(origin.hub(), relay, origin, FIRST_POLICY_EPOCH, EXPIRES)
            .is_err()
    );
}

#[test]
fn responses_cannot_be_substituted_between_calls() {
    let fixture = Fixture::new();
    let call = fixture.call();
    let reply = Reply::Terminal {
        terminal: fixture.export.catalogue.payload.terminals[0].clone(),
    };
    let response = Response::new(&call.payload.request, fixture.origin.identity.hub(), reply);
    let signed = Signed::new(
        ExecutionReply {
            call_fingerprint: call.fingerprint().unwrap(),
            response,
        },
        Purpose::Response,
        &fixture.origin,
    )
    .unwrap();
    signed.validate_for(&call).unwrap();
    assert!(signed.validate_for(&fixture.call()).is_err());
}

#[test]
fn terminal_attestations_fence_subscription_sequence_and_clock() {
    let fixture = Fixture::new();
    let target = fixture.export.catalogue.payload.terminals[0]
        .fields()
        .runtime
        .unwrap()
        .target;
    let request = Request::new(
        fixture.origin.identity.hub(),
        Command::Attach {
            request: AttachRequest::new(target, None).unwrap(),
        },
    )
    .unwrap();
    let call = Signed::new(
        Call {
            request,
            authority: fixture.export.authority.clone(),
            issued_at_ms: NOW,
            expires_at_ms: EXPIRES,
        },
        Purpose::Call,
        &fixture.entry,
    )
    .unwrap();
    let event = TerminalEvent::Heartbeat {
        cursor: OutputCursor {
            target,
            after_sequence: 0,
        },
    };
    let update = Signed::new(
        TerminalUpdate {
            call_fingerprint: call.fingerprint().unwrap(),
            sequence: FIRST_STREAM_SEQUENCE,
            issued_at_ms: NOW,
            event,
        },
        Purpose::Terminal,
        &fixture.origin,
    )
    .unwrap();
    update
        .validate_for(&call, FIRST_STREAM_SEQUENCE, NOW)
        .unwrap();
    assert!(
        update
            .validate_for(&call, FIRST_STREAM_SEQUENCE + STEP, NOW)
            .is_err()
    );
    assert!(
        update
            .validate_for(
                &call,
                FIRST_STREAM_SEQUENCE,
                NOW + MAX_TERMINAL_LIFETIME_MS + STEP
            )
            .is_err()
    );
    assert!(
        update
            .validate_for(&fixture.call(), FIRST_STREAM_SEQUENCE, NOW)
            .is_err()
    );
}
