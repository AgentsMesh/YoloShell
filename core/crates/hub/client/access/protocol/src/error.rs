use crate::constants::*;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessError {
    Identity,
    Grant,
    Configuration,
    DuplicateClient,
    DuplicateOrigin,
    Capacity,
    Expired,
}

impl fmt::Display for AccessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Identity => IDENTITY_ERROR,
            Self::Grant => GRANT_ERROR,
            Self::Configuration => CONFIGURATION_ERROR,
            Self::DuplicateClient => DUPLICATE_CLIENT_ERROR,
            Self::DuplicateOrigin => DUPLICATE_ORIGIN_ERROR,
            Self::Capacity => CAPACITY_ERROR,
            Self::Expired => EXPIRED_ERROR,
        })
    }
}

impl std::error::Error for AccessError {}
