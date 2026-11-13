use std::time::Duration;

pub const MAX_CLIENT_CHANNELS: usize = 256;
pub(crate) const AGENT_DIRECTORY: &str = "agents";
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
pub const OPERATION_TIMEOUT: Duration = Duration::from_secs(30);
pub const CONTROL_IDLE_TIMEOUT: Duration = Duration::from_secs(120);
pub(crate) const SOCKET_PROBE_TIMEOUT: Duration = Duration::from_secs(1);
pub(crate) const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
pub(crate) const LIVE_SOCKET: &str = "Hub socket is already serving";
pub(crate) const IDENTITY_MISMATCH: &str = "Hub service and local backend identities differ";
pub(crate) const WRONG_LOCAL_USER: &str = "Hub client does not belong to the current user";
pub(crate) const EXPECTED_HELLO: &str = "Hub channel must begin with its local hello";
pub(crate) const CHANNEL_FAILED: &str = "Hub client channel ended with an error";
pub(crate) const UNEXPECTED_ATTACH_INPUT: &str = "terminal observers cannot send control frames";
pub(crate) const AUDIT_LOCAL_PRINCIPAL: &str = "local";
