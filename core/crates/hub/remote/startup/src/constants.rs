use std::time::Duration;

pub(crate) const DEADLINE: &str = "remote Hub bootstrap deadline expired";
pub(crate) const BUNDLE_MISSING: &str = "remote runtime bundle is not completely installed";
pub(crate) const PIN_MISMATCH: &str =
    "persistent remote Hub identity does not match its enrollment pin";
pub(crate) const PEER_SOCKET_UNAVAILABLE: &str = "remote Hub peer socket is not published";
pub(crate) const CONFIGURATION_CONFLICT: &str =
    "remote Hub configuration conflicts with the enrollment request";
pub(crate) const CONFIGURATION_UNKNOWN: &str =
    "remote enrollment configuration outcome is unknown; inspect before retrying";
pub(crate) const CONFIGURATION_NOT_READY: &str = "remote Hub configuration has not been applied";
pub(crate) const MUTABLE_RELEASE_PATH: &str =
    "Hub state and Host home must not be inside immutable releases";
pub(crate) const UNEXPECTED_REPLY: &str = "remote Hub returned an unexpected bootstrap reply";
pub(crate) const RECONCILE_POLL: Duration = Duration::from_millis(100);
pub(crate) const DYNAMIC_PORT: u16 = 0;
