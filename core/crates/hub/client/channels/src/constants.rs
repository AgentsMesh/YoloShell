use std::time::Duration;

pub(crate) const INPUT_DISPATCH_CAPACITY: usize = 64;
pub(crate) const INPUT_BATCH_WINDOW: Duration = Duration::from_millis(2);
pub(crate) const RECONNECT_DELAY: Duration = Duration::from_secs(1);
pub const RPC_TIMEOUT: Duration = Duration::from_secs(5);
pub const CONTROL_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(30);
pub const MAX_LIFECYCLE_CHANNELS: usize = 4;
pub const MAX_QUERY_CHANNELS: usize = 4;
pub(crate) const QUERY_LIMIT: &str = "Hub creation query limit reached; query was not sent";
pub(crate) const QUERY_TASK_FAILED: &str = "Hub creation query channel owner failed";
pub(crate) const QUERY_TASK_IDENTITY_MISSING: &str = "query task lost its retained request";
pub(crate) const OPERATION_LIMIT: &str =
    "Hub lifecycle request limit reached; command was not sent";
pub(crate) const OPERATION_TASK_FAILED: &str = "Hub lifecycle channel owner failed";
pub(crate) const NOT_CONNECTED: &str = "Hub control channel is not connected; command was not sent";
pub(crate) const STALE_GENERATION: &str = "Hub control generation changed; command was not sent";
pub(crate) const INVALIDATED_REQUEST: &str =
    "queued request expired or its local scope was retired";
