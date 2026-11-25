use std::time::Duration;

pub(crate) const CONNECTION_GROUP: &str = "connection";
pub(crate) const SOCKET_ARGUMENT: &str = "socket";
pub(crate) const ENDPOINT_ARGUMENT: &str = "endpoint";
pub(crate) const HUB_ARGUMENT: &str = "hub";
pub(crate) const CATALOG_TIMEOUT: Duration = Duration::from_secs(10);
pub const PLATFORM_SNAPSHOT_UNSUPPORTED: &str = "this Hub does not advertise platform snapshots";
pub const PLATFORM_EVENTS_UNSUPPORTED: &str = "this Hub does not advertise platform events";
pub const PLATFORM_EVENTS_SUBSCRIBE_UNSUPPORTED: &str =
    "this Hub does not advertise event subscriptions";
pub const UNEXPECTED_EVENT_SUBSCRIPTION_REPLY: &str = "Hub returned a non-subscription reply";
pub const UNEXPECTED_EVENTS_REPLY: &str = "Hub returned an unexpected platform event reply";
pub const UNEXPECTED_SNAPSHOT_REPLY: &str = "Hub returned an unexpected snapshot reply";
pub const DEFAULT_SNAPSHOT_LIMIT: u16 = 128;
pub const MAX_SNAPSHOT_LIMIT: u16 = 128;
pub const DEFAULT_EVENTS_LIMIT: u16 = 64;
pub const MAX_EVENTS_LIMIT: u16 = 256;
