use std::time::Duration;

pub const RETRY_DELAY: Duration = Duration::from_secs(1);
pub(crate) const CONFLICT: &str = "creation recovery conflicts with retained intent";
pub(crate) const CAPACITY: &str = "creation recovery capacity is exhausted";
pub(crate) const UNKNOWN: &str = "Origin cannot establish the creation outcome";
pub(crate) const REJECTED: &str = "Origin rejected the creation query";
pub(crate) const INVALID_RESPONSE: &str = "Creation query response did not match its request";
pub(crate) const UNSUPPORTED: &str = "Creation queries are unsupported";
pub(crate) const INVALID_ENDPOINT: &str = "Creation query endpoint is invalid";
pub(crate) const WRONG_LOCAL_USER: &str = "Creation query peer is not owned by the current user";
