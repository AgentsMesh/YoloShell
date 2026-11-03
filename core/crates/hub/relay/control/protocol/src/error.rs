use crate::constants::*;
use std::{error::Error, fmt};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtocolError {
    UnsupportedSchema,
    InvalidIdentity,
    InvalidRequest,
    UnsupportedAction,
    OriginMismatch,
    ReceiptMismatch,
    ResponseMismatch,
    EmptyFrame,
    FrameTooLarge,
    InvalidFrame,
}
impl fmt::Display for ProtocolError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str(match self {
            Self::UnsupportedSchema => SCHEMA_ERROR,
            Self::InvalidIdentity => IDENTITY_ERROR,
            Self::InvalidRequest => REQUEST_ERROR,
            Self::UnsupportedAction => ACTION_ERROR,
            Self::OriginMismatch => ORIGIN_ERROR,
            Self::ReceiptMismatch => RECEIPT_ERROR,
            Self::ResponseMismatch => RESPONSE_ERROR,
            Self::EmptyFrame => EMPTY_ERROR,
            Self::FrameTooLarge => SIZE_ERROR,
            Self::InvalidFrame => FRAME_ERROR,
        })
    }
}
// Parser diagnostics can include private bytes. Public errors intentionally have no source chain.
impl Error for ProtocolError {}
