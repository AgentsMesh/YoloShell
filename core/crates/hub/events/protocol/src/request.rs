//! Event page and bounded long-poll request envelopes.

use crate::{
    MAX_PLATFORM_EVENT_PAGE, MAX_PLATFORM_EVENT_WAIT_MS, PlatformEvent, PlatformEventCursor,
    PlatformEventStream,
};
use serde::{Deserialize, Serialize};
use yolo_hub_terminal_protocol::PublicError;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListPlatformEventsFields {
    pub schema_version: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<PlatformEventCursor>,
    pub limit: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ListPlatformEventsFields", into = "ListPlatformEventsFields")]
pub struct ListPlatformEventsRequest(ListPlatformEventsFields);

impl ListPlatformEventsRequest {
    pub fn new(fields: ListPlatformEventsFields) -> Result<Self, PublicError> {
        fields.try_into()
    }

    pub fn fields(&self) -> &ListPlatformEventsFields {
        &self.0
    }
}

impl TryFrom<ListPlatformEventsFields> for ListPlatformEventsRequest {
    type Error = PublicError;

    fn try_from(fields: ListPlatformEventsFields) -> Result<Self, Self::Error> {
        if fields.schema_version != crate::PLATFORM_EVENT_SCHEMA_VERSION
            || fields.limit == 0
            || fields.limit > MAX_PLATFORM_EVENT_PAGE
            || fields.after.is_some_and(|after| after.epoch.is_nil())
        {
            return Err(PublicError::InvalidRequest);
        }
        Ok(Self(fields))
    }
}

impl From<ListPlatformEventsRequest> for ListPlatformEventsFields {
    fn from(value: ListPlatformEventsRequest) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListPlatformEventsResult {
    pub stream: PlatformEventStream,
    pub events: Vec<PlatformEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<PlatformEventCursor>,
    /// True when the requested cursor is no longer served: the epoch changed,
    /// the cursor is ahead of the stream, or retained events were evicted.
    /// Returned events are still valid; reconcile with a snapshot.
    pub lost: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubscribeEventsFields {
    pub schema_version: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<PlatformEventCursor>,
    pub limit: u16,
    pub wait_ms: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "SubscribeEventsFields", into = "SubscribeEventsFields")]
pub struct SubscribeEventsRequest(SubscribeEventsFields);

impl SubscribeEventsRequest {
    pub fn new(fields: SubscribeEventsFields) -> Result<Self, PublicError> {
        fields.try_into()
    }

    pub fn fields(&self) -> &SubscribeEventsFields {
        &self.0
    }
}

impl TryFrom<SubscribeEventsFields> for SubscribeEventsRequest {
    type Error = PublicError;

    fn try_from(fields: SubscribeEventsFields) -> Result<Self, Self::Error> {
        if fields.schema_version != crate::PLATFORM_EVENT_SCHEMA_VERSION
            || fields.limit == 0
            || fields.limit > MAX_PLATFORM_EVENT_PAGE
            || fields.wait_ms == 0
            || fields.wait_ms > MAX_PLATFORM_EVENT_WAIT_MS
            || fields.after.is_some_and(|after| after.epoch.is_nil())
        {
            return Err(PublicError::InvalidRequest);
        }
        Ok(Self(fields))
    }
}

impl From<SubscribeEventsRequest> for SubscribeEventsFields {
    fn from(value: SubscribeEventsRequest) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubscribeEventsResult {
    pub stream: PlatformEventStream,
    pub events: Vec<PlatformEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<PlatformEventCursor>,
    /// True when the requested cursor is no longer served; reconcile with a
    /// snapshot before continuing from `next`.
    pub lost: bool,
    /// True when the bounded wait ended with no event and no lost cursor.
    pub timed_out: bool,
}
