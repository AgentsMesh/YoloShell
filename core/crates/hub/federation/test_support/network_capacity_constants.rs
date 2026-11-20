use std::time::Duration;

pub const TEST_TIMEOUT: Duration = Duration::from_secs(45);
pub const PROGRESS_POLL: Duration = Duration::from_millis(10);
pub const OBSERVED_POLL_CYCLES: u32 = 2;
pub const DUPLEX_BYTES: usize = 1024;
pub const TEST_LISTEN_ADDRESS: &str = "127.0.0.1:0";
pub const LOCAL_DIRECTORY: &str = "local";
pub const PEER_DIRECTORY: &str = "peer";
pub const REVISION_STEP: u64 = 1;
pub const SIGNATURE_CORRUPTION_MASK: u8 = 1;
pub const STREAM_BYTES: &[u8] = b"persistent terminal\0\r\n\x1b[32m";
