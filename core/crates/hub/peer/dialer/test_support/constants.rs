use std::time::Duration;
pub const TIMEOUT: Duration = Duration::from_secs(5);
pub const SHORT_TIMEOUT: Duration = Duration::from_millis(30);
pub const BUFFER_BYTES: usize = 65_536;
pub const SOCKET: &str = "/run/user/1000/yolo/hub/peer.sock";
pub const PAYLOAD: &[u8] = b"injected peer\0data";
pub const REJECTED: &str = "injected transport unavailable";
pub const ONE: usize = 1;
pub const PROFILE: &str = "selected-profile";
