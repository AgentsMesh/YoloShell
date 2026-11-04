pub(crate) const QUEUE_CAPACITY: usize = 1;
pub(super) const MISSING: &str = "Shared layout does not exist";
pub(super) const INVALID: &str = "Hub returned an invalid shared layout revision";
pub(super) const CONFLICT: &str =
    "Another client changed the layout; loaded its revision. Terminals remain running.";
pub(super) const DIRTY_RESTART: &str = "The shared layout changed while local edits were unconfirmed. Reopen with --layout to select its published revision. Terminals remain running.";
