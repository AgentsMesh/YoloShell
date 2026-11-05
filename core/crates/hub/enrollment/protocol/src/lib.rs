//! Signed, expiring enrollment invitations for Hub pairing.
mod codec;
mod constants;
mod invite;
mod model;

pub use codec::{decode, encode};
pub use invite::{SignedInvite, SignedInviteFields, mint};
pub use model::{
    InviteClaims, InviteDial, InvitePolicy, InviteRequest, MintedInvite,
};

#[derive(Debug, thiserror::Error)]
pub enum InviteError {
    #[error("{}", constants::INVALID_CLAIMS)]
    Claims,
    #[error("{}", constants::EXPIRED_CODE)]
    Expired,
    #[error("{}", constants::SIGNATURE_INVALID)]
    Signature,
    #[error("{}", constants::ISSUER_MISMATCH)]
    Issuer,
    #[error("{}", constants::INVALID_CODE)]
    Code,
    #[error("{}", constants::SIGNING_FAILED)]
    Signing,
}
