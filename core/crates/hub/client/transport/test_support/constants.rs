use std::time::Duration;

pub const TEST_TIMEOUT: Duration = Duration::from_secs(10);
pub const POLL_INTERVAL: Duration = Duration::from_millis(5);
pub const GRANT_LIFETIME: Duration = Duration::from_secs(120);
pub const SHORT_GRANT: Duration = Duration::from_secs(2);
pub const PROBE_OBSERVATION: Duration = Duration::from_millis(50);
pub const BUFFER_BYTES: usize = 65_536;
pub const PAYLOAD: &[u8] = b"public\0terminal\r\n\x1b[31mdata";
pub const PRIVATE_PEER_USER: &str = "yolo-peer";
pub const DYNAMIC_PORT: u16 = 0;
pub const REVISION_STEP: u64 = 1;
pub const CLOSED_ONCE: usize = 1;
pub const DENIED_SOCKET: &str = "/denied-public-client-socket";
pub const DENIED_PORT: u32 = 1;
pub const DENIED_HOST: &str = "127.0.0.1";
