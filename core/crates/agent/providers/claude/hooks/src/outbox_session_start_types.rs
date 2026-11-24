use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PreparedSessionStart {
    pub accepted: super::types::AuthenticatedSessionStart,
    pub authority_before: crate::sha256_digest::Sha256Digest,
    pub authority_after: crate::sha256_digest::Sha256Digest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionStartPreparation {
    Apply,
    Commit,
    Committed,
    Established,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionStartAcceptance {
    Recorded,
    Replay,
    Established,
}
