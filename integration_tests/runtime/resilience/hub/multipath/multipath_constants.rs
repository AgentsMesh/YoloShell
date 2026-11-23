use std::time::Duration;

pub const ENTRY_DIRECTORY: &str = "multipath-entry";
pub const RELAY_DIRECTORY: &str = "multipath-relay";
pub const ORIGIN_DIRECTORY: &str = "multipath-origin";
pub const AGENT_NAME: &str = "multipath-origin-agent";
pub const SINGLE_ORIGIN_AGENT: usize = 1;
pub const MAX_PROXY_STREAMS: usize = 4;
pub const ACTIVE_STREAM_STEP: usize = 1;
pub const STABLE_DIRECTORY_WINDOW: Duration = Duration::from_secs(6);
