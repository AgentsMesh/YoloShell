use std::time::Duration;
pub const DEADLINE: Duration = Duration::from_secs(2);
pub const SHORT_DEADLINE: Duration = Duration::from_millis(5);
pub const INVALID_OWNER_DATA: &[u8] = b"unexpected lifetime data";
