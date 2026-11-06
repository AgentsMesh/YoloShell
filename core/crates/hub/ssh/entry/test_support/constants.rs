use std::time::Duration;

pub const TEST_TIMEOUT: Duration = Duration::from_secs(10);
pub const POLL: Duration = Duration::from_millis(5);
pub const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(1);
pub const GRANT_LIFETIME: Duration = Duration::from_secs(120);
pub const HOST: &str = "127.0.0.1";
pub const USERNAME: &str = "test-public-user";
pub const SOCKET: &str = "/private/yolo-test/public.sock";
pub const WRONG_SOCKET: &str = "/private/yolo-test/wrong.sock";
pub const PAYLOAD: &[u8] = b"public-terminal\0\r\n\x1b[31mdata";
pub const BRIDGE_BUFFER: usize = 65_536;
pub const ECHO_BUFFER: usize = 4_096;
pub const COUNT_ONE: usize = 1;
pub const NO_COUNT: usize = 0;
pub const PRIVATE_PORT: u32 = 1;
pub const DISCONNECT_REASON: &str = "test disconnect";
pub const LANGUAGE: &str = "en";
pub const CREATION_CWD: &str = "/";
