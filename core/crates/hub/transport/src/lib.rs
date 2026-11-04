//! Transport-independent public framing; this layer never grants peer authority.
mod codec;
mod constants;
mod error;
mod framed;

pub use codec::{decode, encode};
pub use constants::{FRAME_LENGTH_BYTES, MAX_FRAME_BYTES};
pub use error::TransportError;
pub use framed::{Framed, Incoming};
