#[derive(Debug, Clone, Copy, Eq, PartialEq, thiserror::Error)]
pub enum DelegationError {
    #[error("invalid Hub signing identity")]
    Identity,
    #[error("invalid delegation claims")]
    Claims,
    #[error("delegation is expired or not yet valid")]
    Expired,
    #[error("delegation does not match the pinned origin")]
    Origin,
    #[error("delegation does not authorize this recipient")]
    Recipient,
    #[error("delegation signature is invalid")]
    Signature,
    #[error("delegation exceeds its parent authority")]
    Escalation,
    #[error("delegation path is invalid or cyclic")]
    Path,
    #[error("origin policy has revoked this delegation generation")]
    Revoked,
    #[error("delegation could not be signed or encoded")]
    Signing,
}
