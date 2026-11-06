use std::time::Duration;

pub(crate) const STATUS_TIMEOUT: Duration = Duration::from_secs(5);
pub(crate) const STOP_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const START_TIMEOUT: Duration = Duration::from_secs(30);
pub(crate) const STOP_POLL: Duration = Duration::from_millis(50);
pub(crate) const STOP_TIMED_OUT: &str =
    "local service did not exit after SIGTERM; inspect the exact PID before retrying";
pub(crate) const SIGNAL_FAILED: &str = "could not signal the local service process";
pub(crate) const STOPPED_NOT_STARTED: &str =
    "local services stopped but the new Host/Hub did not start";
pub(crate) const STATE_INCOMPATIBLE: &str = "local Hub state is incompatible with the installed image; back it up or migrate before restarting";
pub(crate) const HUB_SERVICE: &str = "hub";
pub(crate) const HOST_SERVICE: &str = "host";
pub(crate) const CONNECT_TIMEOUT: &str = "local Hub connection timed out";
pub(crate) const CONNECT_FAILED: &str = "local Hub connection failed";
pub(crate) const CONNECT_CREDENTIALS: &str = "local Hub socket credentials are unavailable";
pub(crate) const CONNECT_UID_MISMATCH: &str = "local Hub socket is owned by another user";
pub(crate) const CONNECT_PID_UNAVAILABLE: &str = "local Hub socket peer process is unavailable";
