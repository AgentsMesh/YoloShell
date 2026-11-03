use std::time::Duration;

pub const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
pub const ORIGIN: &str = "http://127.0.0.1:3000";
pub const TIMEOUT: Duration = Duration::from_secs(5);
pub const LIFETIME: Duration = Duration::from_secs(30);
pub const SHORT_LIFETIME: Duration = Duration::from_millis(600);
pub const OUTPUT_SATURATION_WAIT: Duration = Duration::from_millis(100);
pub const CLOSE_POLL_INTERVAL: Duration = Duration::from_millis(5);
pub const TERMINAL_BYTES: &[u8] = b"\x1b]2;agent title\x07\x00\xff\nhello";
pub const CLIENT_ID_FIELD: &str = "client_id";
pub const HELLO_FIELD: &str = "hello";
pub const HUB_ID_FIELD: &str = "hub_id";
pub const METHODS_FIELD: &str = "methods";
pub const GET_CLIENT_ACCESS_METHOD: &str = "get_client_access";
pub const LOOPBACK_BIND: &str = "127.0.0.1:0";
pub const PAUSED_HUB_CAPACITY: usize = 1;
