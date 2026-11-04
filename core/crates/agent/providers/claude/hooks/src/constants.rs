#[cfg(feature = "adversarial-test-hooks")]
pub const CLAUDE_PRE_BEGIN_RELEASE_ENV: &str = "YOLO_TEST_CLAUDE_PRE_BEGIN_RELEASE";
#[cfg(feature = "adversarial-test-hooks")]
pub const CLAUDE_POST_BEGIN_RELEASE_ENV: &str = "YOLO_TEST_CLAUDE_POST_BEGIN_RELEASE";
pub const CLAUDE_WATCHDOG_LAUNCH_SCHEMA_VERSION: u32 = 2;
pub const MAX_CLAUDE_WATCHDOG_LAUNCH_FRAME_BYTES: u64 = 2 * 1024;
pub const CLAUDE_WATCHDOG_LAUNCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);
pub const CLAUDE_WATCHDOG_LAUNCH_SETTLEMENT_TIMEOUT: std::time::Duration =
    std::time::Duration::from_secs(2);
pub const CLAUDE_TERMINATION_GRACE: std::time::Duration = std::time::Duration::from_secs(2);
pub const CLAUDE_KILL_SETTLEMENT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);
pub const CLAUDE_TERMINATION_POLL_INTERVAL: std::time::Duration =
    std::time::Duration::from_millis(100);
pub const CLAUDE_RETAINED_TREE_REFRESH_INTERVAL: std::time::Duration =
    CLAUDE_TERMINATION_POLL_INTERVAL;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[doc(hidden)]
pub const CLAUDE_WATCHDOG_TEST_FAILURE_ENV: &str = "YOLO_TEST_CLAUDE_WATCHDOG_LAUNCH_FAILURE";
#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[doc(hidden)]
pub const CLAUDE_WATCHDOG_TEST_WATCHDOG_ARMED: &str = "watchdog_armed";
#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[doc(hidden)]
pub const CLAUDE_WATCHDOG_TEST_LAUNCHER_ARMED: &str = "launcher_armed";
#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[doc(hidden)]
pub const CLAUDE_WATCHDOG_TEST_OUTER_READY: &str = "outer_ready";
pub const CLAUDE_HOOK_OUTBOX_DIRECTORY: &str = "claude-hook-outbox";
pub const CLAUDE_HOOK_OUTBOX_FILE: &str = "state.json";
pub const CLAUDE_HOOK_OUTBOX_SCHEMA_VERSION: u32 = 4;
pub const INITIAL_CLAUDE_ACKNOWLEDGED_SEQUENCE: u64 = 0;
pub const MAX_CLAUDE_OUTBOX_INVOCATIONS: usize = 64;
pub const MAX_CLAUDE_FAILED_HANDSHAKE_AUDITS: usize = 64;
pub const MAX_CLAUDE_CONCURRENT_HANDSHAKE_SETTLEMENTS: usize = 4;
const _: () = assert!(MAX_CLAUDE_CONCURRENT_HANDSHAKE_SETTLEMENTS > 0);
pub const MAX_CLAUDE_OUTBOX_EVENTS: usize = 64;
pub const MAX_CLAUDE_OUTBOX_BYTES: u64 = 1024 * 1024;
pub const CLAUDE_OUTBOX_LOCK_RETRY_MILLIS: u64 = 1;
pub const CLAUDE_OUTBOX_LOCK_TIMEOUT_SECS: u64 = 8;
pub const CLAUDE_OUTBOX_LOCK_RETRY: std::time::Duration =
    std::time::Duration::from_millis(CLAUDE_OUTBOX_LOCK_RETRY_MILLIS);
pub const CLAUDE_OUTBOX_LOCK_TIMEOUT: std::time::Duration =
    std::time::Duration::from_secs(CLAUDE_OUTBOX_LOCK_TIMEOUT_SECS);

pub(crate) mod outbox_error {
    pub const ACKNOWLEDGEMENT_ORDER: &str = "Claude hook outbox acknowledgement is out of order";
    pub const AGENT_CHANGED: &str = "Claude outbox agent process changed";
    pub const RECOVERY_CHANGED: &str =
        "recovered Claude lifecycle queue changed before acknowledgement";
    pub const RECOVERY_OWNER: &str =
        "recovered Claude lifecycle queue still has a live or unknown owner";
    pub const RECOVERY_QUEUE_MISSING: &str = "recovered Claude lifecycle queue is missing";
    pub const BEGIN_REQUEST: &str = "Claude outbox contains a begin request";
    pub const CREDENTIALS_CHANGED: &str = "Claude outbox credentials changed";
    pub const DATA_HOME_MISSING: &str = "Claude data home is unavailable for outbox evidence";
    pub const DELIVERY_CLAIM_LOST: &str = "Claude hook outbox delivery claim was lost";
    pub const EVENT_CAPACITY: &str = "Claude hook outbox event capacity is exhausted";
    pub const EVENT_SEQUENCE_OVERFLOW: &str = "Claude hook event sequence overflow";
    pub const INVOCATION_CAPACITY: &str = "Claude hook outbox invocation capacity is exhausted";
    pub const PARENT_PROCESS_MISSING: &str = "Claude hook parent process is unavailable";
    pub const SEQUENCE_NOT_RESERVED: &str = "Claude hook outbox sequence was not reserved";
    pub const SETTINGS_PARENT_MISSING: &str = "Claude settings directory has no session parent";
    pub const STATE_INVALID: &str = "Claude hook outbox state is invalid";
    pub const STATE_TOO_LARGE: &str = "Claude hook outbox is too large";
    pub const SESSION_START_AUTHORITY_CHANGED: &str =
        "Claude SessionStart authority changed across its durable transaction";
    pub const SESSION_START_TRANSACTION_CONFLICT: &str =
        "Claude SessionStart durable transaction conflicts with another event";
    pub const SESSION_START_TRANSACTION_MISSING: &str =
        "Claude SessionStart durable transaction is missing";
    pub const TRANSCRIPT_IDENTITY_MISSING: &str =
        "Claude outbox transcript identity is unavailable";
}
