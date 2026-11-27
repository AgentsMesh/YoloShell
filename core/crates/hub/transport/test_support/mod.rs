use serde::{Deserialize, Serialize};
use tokio::time::{Duration, Instant};

pub const DEADLINE: Duration = Duration::from_secs(2);
pub const PIPE_CAPACITY: usize = 32;
pub const MESSAGE_SEQUENCE: u64 = 19;
pub const TEST_BYTES: &[u8] = b"\x00\x01\xff\x1b[1;1Hterminal";

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub sequence: u64,
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

pub fn deadline() -> Instant {
    Instant::now() + DEADLINE
}
pub fn message() -> Message {
    Message {
        sequence: MESSAGE_SEQUENCE,
        data: TEST_BYTES.into(),
    }
}
