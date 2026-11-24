pub(crate) const INVALID: &str = "remote setup request does not match this Hub or profile";
pub(crate) const UNEXPECTED: &str = "remote setup received an unexpected Hub reply";
pub(crate) const JOURNAL_CHANGED: &str = "remote setup storage or broker directory changed";
pub(crate) const OPERATION_FAILED: &str = "remote setup stopped without a confirmed outcome";
pub(crate) const OPERATION_TIMED_OUT: &str =
    "remote setup deadline expired without a confirmed outcome";
pub(crate) const MAX_ACTIVE_SETUPS: usize = 16;
pub(crate) const MAX_RETAINED_SETUP_RESULTS: usize = 64;
