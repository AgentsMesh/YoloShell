pub const RPC_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
pub const AGENT_LIST_UNSUPPORTED: &str = "this Hub does not advertise agent observation";
pub const AGENT_NOT_FOUND: &str = "no agent matches the selector";
pub const SELECTOR_AMBIGUOUS: &str =
    "the selector matches more than one terminal; use a unique name or id";
pub const NO_CURRENT_AGENT: &str = "the selected terminal has no current agent";
pub const INVALID_RUN_ID: &str = "the run id is not a valid non-nil UUID";
pub const RUN_NOT_FOUND: &str = "agent run was not found";
pub const RUN_QUERIES_UNSUPPORTED: &str = "Hub does not support agent run queries";
pub const INVALID_LABEL_FILTER: &str = "label filter must be key=value";
pub const UNEXPECTED_REPLY: &str = "Hub returned an unexpected reply";
