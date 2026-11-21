//! Bounded platform-event buffer. Events are derived from successful public
//! effects; the buffer is per Hub process and never survives a restart.

mod source;

use source::kinds_for;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::watch;
use tokio::time::Instant;
use uuid::Uuid;
use yolo_hub_events_protocol::{
    ListPlatformEventsRequest, ListPlatformEventsResult, MAX_PLATFORM_EVENT_BUFFER,
    PlatformEvent, PlatformEventCursor, PlatformEventKind, PlatformEventStream,
    SubscribeEventsRequest, SubscribeEventsResult,
};
use yolo_hub_rpc_protocol::{Command, Reply};

/// How often a subscription rescans without an append hint. The watch channel
/// carries every append; this only guards a missed notification.
const RESCAN: Duration = Duration::from_secs(1);

pub struct EventBus {
    state: Mutex<State>,
    wake: watch::Sender<u64>,
}

struct State {
    epoch: Uuid,
    next_sequence: u64,
    events: VecDeque<PlatformEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (wake, _) = watch::channel(0);
        Self {
            state: Mutex::new(State {
                epoch: Uuid::new_v4(),
                next_sequence: 1,
                events: VecDeque::new(),
            }),
            wake,
        }
    }

    pub fn record(&self, command: &Command, reply: &Reply) {
        for kind in kinds_for(command, reply) {
            self.append(kind);
        }
    }

    fn append(&self, kind: PlatformEventKind) {
        let mut state = self.lock();
        let sequence = state.next_sequence;
        state.next_sequence += 1;
        state.events.push_back(PlatformEvent {
            sequence,
            at_ms: now_ms(),
            kind,
        });
        while state.events.len() > MAX_PLATFORM_EVENT_BUFFER {
            state.events.pop_front();
        }
        let _ = self.wake.send(sequence);
    }

    #[cfg(test)]
    pub(crate) fn append_for_test(&self, kind: PlatformEventKind) {
        self.append(kind);
    }

    pub fn list(&self, request: &ListPlatformEventsRequest) -> ListPlatformEventsResult {
        self.page(request.fields().after, request.fields().limit)
    }

    /// Wait until events newer than `after` exist, the cursor is lost, or the
    /// caller's deadline passes. The page semantics match `list`, so a
    /// client can resume from the same cursor shape.
    pub async fn subscribe(
        &self,
        request: &SubscribeEventsRequest,
        deadline: Instant,
    ) -> SubscribeEventsResult {
        let fields = request.fields();
        let mut wake = self.wake.subscribe();
        loop {
            let page = self.page(fields.after, fields.limit);
            if !page.events.is_empty() || page.lost {
                return SubscribeEventsResult {
                    stream: page.stream,
                    events: page.events,
                    next: page.next,
                    lost: page.lost,
                    timed_out: false,
                };
            }
            let now = Instant::now();
            if now >= deadline {
                return SubscribeEventsResult {
                    stream: page.stream,
                    events: Vec::new(),
                    next: page.next,
                    lost: false,
                    timed_out: true,
                };
            }
            tokio::select! {
                _ = wake.changed() => {}
                _ = tokio::time::sleep_until((now + RESCAN).min(deadline)) => {}
            }
        }
    }

    fn page(
        &self,
        after: Option<PlatformEventCursor>,
        limit: u16,
    ) -> ListPlatformEventsResult {
        let state = self.lock();
        let latest = state.next_sequence.saturating_sub(1);
        let first_sequence = state
            .events
            .front()
            .map(|event| event.sequence)
            .unwrap_or(state.next_sequence);
        let mut lost = false;
        let start = match after {
            None => 0,
            Some(cursor) if cursor.epoch != state.epoch => {
                lost = true;
                0
            }
            Some(cursor) if cursor.sequence > latest => {
                lost = true;
                latest
            }
            Some(cursor) => {
                if cursor.sequence.saturating_add(1) < first_sequence {
                    lost = true;
                }
                cursor.sequence
            }
        };
        let mut events: Vec<PlatformEvent> = state
            .events
            .iter()
            .filter(|event| event.sequence > start)
            .cloned()
            .collect();
        events.truncate(limit as usize);
        let next = events
            .last()
            .map(|event| PlatformEventCursor {
                epoch: state.epoch,
                sequence: event.sequence,
            })
            .or_else(|| {
                lost.then_some(PlatformEventCursor {
                    epoch: state.epoch,
                    sequence: latest,
                })
            })
            .or(after);
        ListPlatformEventsResult {
            stream: PlatformEventStream {
                epoch: state.epoch,
                sequence: latest,
                first_sequence,
            },
            events,
            next,
            lost,
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
