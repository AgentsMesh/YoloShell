//! Pure wire contracts for atomic runtime publication; contains no filesystem operations.
mod constants;
mod inspection;
mod path;
mod receipt;
mod request;
mod selection;
mod wire;

pub use constants::*;
pub use inspection::{InspectionFields, InspectionReceipt, SelectedRelease, SelectedReleaseFields};
pub use path::validate_install_root;
pub use receipt::{PublicationReceipt, PublicationReceiptFields};
pub use request::{PublicationRequest, PublicationRequestFields};
pub use selection::{Selection, SelectionFields};
pub use wire::{
    decode_inspection, decode_receipt, decode_request, decode_request_argument, decode_selection,
    encode_inspection, encode_receipt, encode_request, encode_request_argument,
};
