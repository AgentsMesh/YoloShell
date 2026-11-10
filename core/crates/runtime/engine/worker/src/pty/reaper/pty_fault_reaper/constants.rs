use std::time::Duration;

pub(crate) const PTY_FAULT_REAPER_THREAD_NAME: &str = "yolo-pty-fault-reaper";
pub(crate) const PRIMARY_HANDOFF_FAILURE_DETAIL: &str =
    "primary PTY fault reaper stopped before custody handoff";
pub(crate) const POST_COMPLETION_FAULT_DETAIL: &str =
    "PTY fault cleanup failed after completion publication";
pub(crate) const CUSTODY_GONE_PENDING: &str = "gone_pending";
pub(crate) const CUSTODY_GONE_REPORTED: &str = "gone_reported";
pub(crate) const CUSTODY_MASTER_CLOSE_PENDING: &str = "master_close_pending";
pub(crate) const PTY_FAULT_REAPER_RETRY_INITIAL_INTERVAL: Duration = Duration::from_millis(10);
pub(crate) const PTY_FAULT_REAPER_RETRY_MAX_INTERVAL: Duration = Duration::from_secs(2);
pub(crate) const PTY_FAULT_REAPER_RETRY_MULTIPLIER: u32 = 2;
