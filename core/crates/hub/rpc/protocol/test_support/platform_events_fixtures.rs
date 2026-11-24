use crate::fixtures::{constants::*, *};
use uuid::Uuid;
use yolo_hub_events_protocol::{
    ListPlatformEventsFields, ListPlatformEventsRequest, ListPlatformEventsResult,
    PLATFORM_EVENT_SCHEMA_VERSION, PlatformEvent, PlatformEventCursor, PlatformEventKind,
    PlatformEventStream, SubscribeEventsFields, SubscribeEventsRequest, SubscribeEventsResult,
};
use yolo_hub_rpc_protocol::{Command, Reply};

const EVENT_EPOCH: u128 = 9;
const EVENT_SEQUENCE: u64 = 1;
const EVENT_LIMIT: u16 = 16;
const EVENT_WAIT_MS: u32 = 10;

pub fn samples() -> Vec<(Command, Reply)> {
    let epoch = Uuid::from_u128(EVENT_EPOCH);
    let stream = PlatformEventStream {
        epoch,
        sequence: EVENT_SEQUENCE,
        first_sequence: EVENT_SEQUENCE,
    };
    let events = vec![PlatformEvent {
        sequence: EVENT_SEQUENCE,
        at_ms: 0,
        kind: PlatformEventKind::TerminalCreated {
            address: summary(HUB_ID, SESSION_ID).fields().address,
        },
    }];
    let next = Some(PlatformEventCursor {
        epoch,
        sequence: EVENT_SEQUENCE,
    });
    vec![
        (
            Command::ListPlatformEvents {
                request: ListPlatformEventsRequest::new(ListPlatformEventsFields {
                    schema_version: PLATFORM_EVENT_SCHEMA_VERSION,
                    after: None,
                    limit: EVENT_LIMIT,
                })
                .unwrap(),
            },
            Reply::PlatformEvents {
                events: ListPlatformEventsResult {
                    stream: stream.clone(),
                    events: events.clone(),
                    next,
                    lost: false,
                },
            },
        ),
        (
            Command::SubscribeEvents {
                request: SubscribeEventsRequest::new(SubscribeEventsFields {
                    schema_version: PLATFORM_EVENT_SCHEMA_VERSION,
                    after: None,
                    limit: EVENT_LIMIT,
                    wait_ms: EVENT_WAIT_MS,
                })
                .unwrap(),
            },
            Reply::EventSubscription {
                subscription: SubscribeEventsResult {
                    stream,
                    events,
                    next,
                    lost: false,
                    timed_out: false,
                },
            },
        ),
    ]
}
