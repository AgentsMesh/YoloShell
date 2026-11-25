use crate::*;
use uuid::Uuid;
use yolo_hub_terminal_protocol::PublicError;

const LIMIT: u16 = 16;
const WAIT_MS: u32 = 100;

fn subscribe(
    after: Option<PlatformEventCursor>,
    limit: u16,
    wait_ms: u32,
) -> Result<SubscribeEventsRequest, PublicError> {
    SubscribeEventsRequest::new(SubscribeEventsFields {
        schema_version: PLATFORM_EVENT_SCHEMA_VERSION,
        after,
        limit,
        wait_ms,
    })
}

#[test]
fn layout_changed_round_trips_with_its_origin() {
    let kind = PlatformEventKind::LayoutChanged {
        address: yolo_presentation_protocol::LayoutAddress {
            hub_id: yolo_hub_identity::HubId::new(),
            layout_id: Uuid::new_v4(),
        },
        revision: 3,
    };
    assert_eq!(kind.hub_id(), Some(match kind {
        PlatformEventKind::LayoutChanged { address, .. } => address.hub_id,
        _ => unreachable!(),
    }));
    let wire = serde_json::to_vec(&kind).unwrap();
    assert_eq!(
        serde_json::from_slice::<PlatformEventKind>(&wire).unwrap(),
        kind
    );
}

#[test]
fn subscriptions_bound_the_wait_and_reject_a_nil_epoch() {
    assert!(subscribe(None, LIMIT, WAIT_MS).is_ok());
    assert_eq!(subscribe(None, LIMIT, 0), Err(PublicError::InvalidRequest));
    assert_eq!(
        subscribe(None, LIMIT, MAX_PLATFORM_EVENT_WAIT_MS + 1),
        Err(PublicError::InvalidRequest)
    );
    assert_eq!(subscribe(None, 0, WAIT_MS), Err(PublicError::InvalidRequest));
    let cursor = PlatformEventCursor {
        epoch: Uuid::nil(),
        sequence: 0,
    };
    assert_eq!(
        subscribe(Some(cursor), LIMIT, WAIT_MS),
        Err(PublicError::InvalidRequest)
    );
}

#[test]
fn subscriptions_round_trip_their_wire_shape() {
    let cursor = PlatformEventCursor {
        epoch: Uuid::new_v4(),
        sequence: 2,
    };
    let request = subscribe(Some(cursor), LIMIT, WAIT_MS).unwrap();
    let wire = serde_json::to_vec(&request).unwrap();
    assert_eq!(
        serde_json::from_slice::<SubscribeEventsRequest>(&wire).unwrap(),
        request
    );
    let result = SubscribeEventsResult {
        stream: PlatformEventStream {
            epoch: cursor.epoch,
            sequence: 2,
            first_sequence: 1,
        },
        events: Vec::new(),
        next: Some(cursor),
        lost: false,
        timed_out: true,
    };
    let wire = serde_json::to_vec(&result).unwrap();
    assert_eq!(
        serde_json::from_slice::<SubscribeEventsResult>(&wire).unwrap(),
        result
    );
}
