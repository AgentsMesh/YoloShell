use std::time::Duration;
pub const TIMEOUT: Duration = Duration::from_secs(5);
pub const SHORT_TIMEOUT: Duration = Duration::from_millis(30);
pub const BUFFER_BYTES: usize = 65_536;
pub const PORT: u16 = 43123;
pub const DYNAMIC_PORT: u16 = 0;
pub const HOST: &str = "factory-only.invalid";
pub const PAYLOAD: &[u8] = b"injected peer\0data";
pub const REJECTED: &str = "injected transport unavailable";
pub const ONE: usize = 1;
pub const PROFILE: &str = "selected-profile";
pub const PROFILE_REVISION: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
