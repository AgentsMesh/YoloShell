pub(crate) const TOGGLE_KEY: char = 'm';
pub(super) const MAX_VIEWS: usize = 32;
pub(super) const QUERY_CAPACITY: usize = 2;
pub(super) const POLL: std::time::Duration = std::time::Duration::from_millis(500);
pub(super) const NO_AGENT: &str = "Chat requires a current agent in this pane";
pub(super) const CACHE_FULL: &str =
    "Chat drafts are retained; clear an unused draft before opening another chat";
pub(super) const STALE: &str = "Agent execution changed; return to its terminal";
pub(super) const REFRESHING: &str = "Refreshing agent state; the draft is retained";
pub(super) const RETIRED: &str =
    "Previous agent Chat is read-only · Ctrl-x discards its local draft · Esc terminal";
pub(super) const NEED_CONTROL: &str =
    "Append & send requires terminal input control; Ctrl-b a acquires it";
