use crate::constants::*;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildImageError {
    UnsupportedSchema,
    InvalidDigest,
    InvalidSize,
}

impl fmt::Display for BuildImageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedSchema => INVALID_SCHEMA,
            Self::InvalidDigest => INVALID_DIGEST,
            Self::InvalidSize => INVALID_SIZE,
        })
    }
}
impl std::error::Error for BuildImageError {}
