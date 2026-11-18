//! Private desktop/relay Agent control correlation and bounded typed Cap'n Proto messages.
//! No writer secret, socket ownership, dispatch authority or terminal input lives here.
//! Transport owners use a bounded length prefix. A malformed reply after possible dispatch means Unknown to
//! the caller; these codecs never establish that an operation was not sent.

mod capnp_request;
mod capnp_response;
mod codec;
mod constants;
mod error;
mod outcome;
mod protocol;
mod request;
mod response;
mod status;

pub use codec::{decode_request, decode_response, encode_request, encode_response};
pub use constants::*;
pub use error::ProtocolError;
pub use protocol::{NotSentReason, Outcome};
pub use request::{Request, RequestFields};
pub use response::{Response, ResponseFields};
pub use status::{Status, StatusKind, decode_status, encode_status};
