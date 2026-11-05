//! One-message enrollment exchange carried inside an authenticated SSH channel.
mod error;
mod frame;
mod model;

pub use error::WireError;
pub use frame::{read_outcome, read_request, write_outcome, write_request};
pub use model::{JoinErrorCode, JoinOutcome, JoinReply, JoinRequest, JOIN_SCHEMA_VERSION};

pub const MAX_JOIN_FRAME_BYTES: usize = 64 * 1024;
