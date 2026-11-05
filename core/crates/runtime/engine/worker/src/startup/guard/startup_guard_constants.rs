pub(crate) const STARTUP_MONITOR_UNAVAILABLE_CONTEXT: &str =
    "startup process-scope monitor is unavailable";
pub(crate) const STARTUP_TREE_UNAVAILABLE_CONTEXT: &str = "startup process tree is unavailable";
pub(crate) const STARTUP_CHILD_UNAVAILABLE_CONTEXT: &str = "startup child is unavailable";
pub(crate) const STARTUP_AUTHORITY_UNAVAILABLE_CONTEXT: &str =
    "startup child wait authority is unavailable";
pub(crate) const STARTUP_OUTPUT_PROGRESS_UNAVAILABLE_CONTEXT: &str =
    "startup PTY output-progress authority is unavailable";
#[cfg(test)]
pub(crate) const STARTUP_READER_NOT_INSTALLED_CONTEXT: &str =
    "durable PTY reader is not installed before ownership transfer";
pub(crate) const STARTUP_READER_ALREADY_INSTALLED_CONTEXT: &str =
    "durable PTY reader is already installed";
pub(crate) const ACTIVE_OBSERVER_DETAIL: &str = "startup process tree still has an active observer";
pub(crate) const CHILD_IDENTITY_MISMATCH_DETAIL: &str =
    "owned PTY child does not match its captured root";
pub(crate) const STARTUP_CLEANUP_FAILURE_DETAIL: &str =
    "failed to terminate child after worker startup failure";
pub(crate) const STARTUP_CLEANUP_PANIC_DETAIL: &str =
    "contained panic during startup child cleanup";
pub(crate) const STARTUP_FAULT_REAPER_UNAVAILABLE_CONTEXT: &str =
    "startup PTY fault reaper reservation is unavailable";
