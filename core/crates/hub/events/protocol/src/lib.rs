//! Sequenced platform events for cursor-based observation. Events are derived
//! from confirmed public effects and carry no PTY bytes or lifecycle authority.

mod event;
mod request;

pub use event::{
    PlatformEvent, PlatformEventCursor, PlatformEventKind, PlatformEventStream,
};
pub use request::{
    ListPlatformEventsFields, ListPlatformEventsRequest, ListPlatformEventsResult,
    SubscribeEventsFields, SubscribeEventsRequest, SubscribeEventsResult,
};

pub const PLATFORM_EVENT_SCHEMA_VERSION: u16 = 1;
pub const MAX_PLATFORM_EVENT_PAGE: u16 = 256;
pub const MAX_PLATFORM_EVENT_BUFFER: usize = 512;
pub const MAX_PLATFORM_EVENT_WAIT_MS: u32 = 1_000;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
