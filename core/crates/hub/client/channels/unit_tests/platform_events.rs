use crate::platform_events::advance;
use uuid::Uuid;
use yolo_hub_events_protocol::{PlatformEventCursor, PlatformEventStream, SubscribeEventsResult};

fn result(
    lost: bool,
    next: Option<PlatformEventCursor>,
    epoch: Uuid,
    sequence: u64,
) -> SubscribeEventsResult {
    SubscribeEventsResult {
        stream: PlatformEventStream {
            epoch,
            sequence,
            first_sequence: 1,
        },
        events: Vec::new(),
        next,
        lost,
        timed_out: !lost && next.is_none(),
    }
}

#[test]
fn lost_cursor_jumps_to_the_stream_head_instead_of_replaying() {
    let epoch = Uuid::new_v4();
    let after = Some(PlatformEventCursor { epoch, sequence: 3 });
    let next = Some(PlatformEventCursor { epoch, sequence: 9 });
    assert_eq!(advance(after, &result(false, next, epoch, 20)), next);
    assert_eq!(
        advance(after, &result(true, next, epoch, 40)),
        Some(PlatformEventCursor { epoch, sequence: 40 })
    );
    assert_eq!(advance(after, &result(false, None, epoch, 0)), after);
}
