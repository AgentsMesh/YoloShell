use std::time::Duration;

/// Allow the five-second wire deadline and completion delivery to settle first.
pub const ATTEMPT_TIMEOUT: Duration = Duration::from_secs(10);
/// Persistent intent is retried at a bounded cadence, without forcing takeover.
pub const RETRY_INTERVAL: Duration = Duration::from_secs(1);
