pub(super) const BIND_ERROR: &str = "bind staged Unix socket";
pub(super) const BIND_DISAPPEARED_ERROR: &str = "staged Unix socket disappeared";
pub(super) const CHMOD_CHANGED_ERROR: &str = "staged Unix socket changed during chmod";
#[cfg(any(test, feature = "persistence-test-support"))]
pub(super) const CLEANUP_HOOK_ALREADY_SET: &str = "socket cleanup hook was already set";
pub(super) const PARENT_MISMATCH_ERROR: &str = "socket path does not match retained parent";
pub(super) const PROBE_ALIAS_CHANGED_ERROR: &str = "socket probe alias identity changed";
pub(super) const PROBE_LINK_COUNT_ERROR: &str = "socket probe link count changed";
pub(super) const STAGE_RESIDUE_UNPROVEN_ERROR: &str =
    "socket publication stage residue could not be proven";
pub(super) const PROBE_ERROR: &str = "probe retained Unix socket";
pub(super) const PROBE_TIMEOUT_ERROR: &str = "retained Unix socket probe timed out";
pub(super) const PUBLISH_CHANGED_ERROR: &str = "published Unix socket identity changed";
pub(super) const PUBLISH_DISAPPEARED_ERROR: &str = "published Unix socket disappeared";
pub(super) const TARGET_CHANGED_ERROR: &str = "retained Unix socket changed before removal";
pub(super) const TARGET_REMOVE_ERROR: &str = "retained Unix socket removal was not durable";
pub(super) const SOCKET_TYPE_ERROR: &str = "retained endpoint is not a Unix socket";
pub(super) const SOCKET_OWNER_ERROR: &str = "retained Unix socket owner mismatch";
pub(super) const SOCKET_MODE_ERROR: &str = "retained Unix socket mode is not private";
pub(super) const LINK_ERROR: &str = "link retained Unix socket probe alias";
pub(super) const RENAME_ERROR: &str = "publish retained Unix socket";
pub(super) const UNLINK_ERROR: &str = "remove retained Unix socket";
pub(super) const CHMOD_ERROR: &str = "secure staged Unix socket";
pub(super) const STAGE_PARENT_CHANGED_ERROR: &str =
    "staged Unix socket does not share the retained target parent";
pub(super) const STAGE_PATH_BUDGET_ERROR: &str =
    "retained socket parent cannot accommodate publication staging";
