use crate::EventBus;
use std::time::Duration;
use uuid::Uuid;
use yolo_hub_events_protocol::{
    ListPlatformEventsFields, ListPlatformEventsRequest, PLATFORM_EVENT_SCHEMA_VERSION,
    PlatformEventCursor, PlatformEventKind, SubscribeEventsFields, SubscribeEventsRequest,
};
use yolo_hub_identity::{HubId, TerminalAddress, TerminalId};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_presentation_protocol::{LayoutAddress, LayoutUpdate};

const PAGE: u16 = 2;
const AHEAD: u64 = 999;
const WAIT_MS: u32 = 1;
const TIMEOUT_MS: u64 = 1;
const APPEND_DELAY_MS: u64 = 10;
const SUBSCRIPTION_SECS: u64 = 5;

fn kind(hub: HubId) -> PlatformEventKind {
    PlatformEventKind::TerminalCreated {
        address: TerminalAddress {
            hub_id: hub,
            terminal_id: TerminalId::from_uuid(Uuid::new_v4()).unwrap(),
        },
    }
}

fn bus(count: usize) -> (EventBus, HubId) {
    let bus = EventBus::new();
    let hub = HubId::new();
    for _ in 0..count {
        bus.append_for_test(kind(hub));
    }
    (bus, hub)
}

fn request(after: Option<PlatformEventCursor>, limit: u16) -> ListPlatformEventsRequest {
    ListPlatformEventsRequest::new(ListPlatformEventsFields {
        schema_version: PLATFORM_EVENT_SCHEMA_VERSION,
        after,
        limit,
    })
    .unwrap()
}

#[test]
fn layout_update_emits_a_layout_changed_event() {
    let hub = HubId::new();
    let change = LayoutUpdate {
        address: LayoutAddress {
            hub_id: hub,
            layout_id: Uuid::new_v4(),
        },
        request_id: Uuid::new_v4(),
        expected_revision: 0,
        tabs: Vec::new(),
    };
    let reply = Reply::Layout {
        document: Some(change.apply(None).unwrap()),
    };
    assert_eq!(
        crate::source::kinds_for(&Command::UpdateLayout { change: change.clone() }, &reply),
        vec![PlatformEventKind::LayoutChanged {
            address: change.address,
            revision: 1,
        }]
    );
}

#[test]
fn events_are_sequenced_and_paginate_by_cursor() {
    let (bus, _hub) = bus(3);
    let first = bus.list(&request(None, PAGE));
    assert_eq!(first.stream.sequence, 3);
    assert_eq!(first.events.len(), 2);
    assert_eq!(first.events[0].sequence, 1);
    assert_eq!(first.events[1].sequence, 2);
    assert!(!first.lost);
    let next = first.next.unwrap();
    assert_eq!(next.sequence, 2);

    let second = bus.list(&request(Some(next), PAGE));
    assert_eq!(second.events.len(), 1);
    assert_eq!(second.events[0].sequence, 3);
    assert_eq!(second.next.unwrap().sequence, 3);

    let idle = bus.list(&request(second.next, PAGE));
    assert!(idle.events.is_empty());
    assert!(!idle.lost);
}

#[test]
fn epoch_mismatch_reports_lost_and_returns_retained_events() {
    let (bus, _hub) = bus(1);
    let cursor = PlatformEventCursor {
        epoch: Uuid::new_v4(),
        sequence: 0,
    };
    let result = bus.list(&request(Some(cursor), PAGE));
    assert!(result.lost);
    assert_eq!(result.events.len(), 1);
}

#[test]
fn evicted_or_ahead_cursors_report_lost() {
    let (bus, _hub) = bus(usize::from(yolo_hub_events_protocol::MAX_PLATFORM_EVENT_BUFFER) + 1);
    let stream = bus.list(&request(None, 1)).stream;
    assert_eq!(stream.first_sequence, 2);

    let evicted = bus.list(&request(
        Some(PlatformEventCursor {
            epoch: stream.epoch,
            sequence: 0,
        }),
        PAGE,
    ));
    assert!(evicted.lost);
    assert_eq!(evicted.events[0].sequence, stream.first_sequence);

    let ahead = bus.list(&request(
        Some(PlatformEventCursor {
            epoch: stream.epoch,
            sequence: AHEAD,
        }),
        PAGE,
    ));
    assert!(ahead.lost);
    assert!(ahead.events.is_empty());
    assert_eq!(ahead.next.unwrap().sequence, stream.sequence);
}

fn subscription(after: Option<PlatformEventCursor>) -> SubscribeEventsRequest {
    SubscribeEventsRequest::new(SubscribeEventsFields {
        schema_version: PLATFORM_EVENT_SCHEMA_VERSION,
        after,
        limit: PAGE,
        wait_ms: WAIT_MS,
    })
    .unwrap()
}

#[tokio::test]
async fn subscription_times_out_without_events() {
    let (bus, _hub) = bus(0);
    let deadline = tokio::time::Instant::now() + Duration::from_millis(TIMEOUT_MS);
    let result = bus.subscribe(&subscription(None), deadline).await;
    assert!(result.timed_out);
    assert!(result.events.is_empty());
    assert!(!result.lost);
}

#[tokio::test]
async fn subscription_wakes_on_append() {
    let bus = std::sync::Arc::new(EventBus::new());
    let hub = HubId::new();
    let request = subscription(None);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(SUBSCRIPTION_SECS);
    let waiter = {
        let bus = bus.clone();
        tokio::spawn(async move { bus.subscribe(&request, deadline).await })
    };
    tokio::time::sleep(Duration::from_millis(APPEND_DELAY_MS)).await;
    bus.append_for_test(kind(hub));
    let result = waiter.await.unwrap();
    assert!(!result.timed_out);
    assert_eq!(result.events.len(), 1);
    assert_eq!(result.events[0].sequence, 1);
}

#[tokio::test]
async fn subscription_reports_a_lost_cursor_immediately() {
    let (bus, _hub) = bus(1);
    let cursor = PlatformEventCursor {
        epoch: Uuid::new_v4(),
        sequence: 0,
    };
    let deadline = tokio::time::Instant::now() + Duration::from_secs(SUBSCRIPTION_SECS);
    let result = bus.subscribe(&subscription(Some(cursor)), deadline).await;
    assert!(result.lost);
    assert!(!result.timed_out);
    assert_eq!(result.events.len(), 1);
}
