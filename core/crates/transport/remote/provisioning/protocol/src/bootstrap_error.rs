use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum BootstrapError {
    #[error("unsupported remote bootstrap schema")]
    Schema,
    #[error("invalid remote bootstrap request identity")]
    RequestIdentity,
    #[error("remote bootstrap must connect distinct Hub identities")]
    SameIdentity,
    #[error("remote bootstrap authorization must include observation and bounded transit")]
    Authorization,
    #[error("remote peer socket must be a canonical absolute Unix socket path")]
    Endpoint,
    #[error("invalid applied remote configuration revision")]
    Revision,
    #[error("remote bootstrap receipt does not match its request")]
    Correlation,
    #[error("remote Hub identity does not match its pin")]
    PeerMismatch,
    #[error("running remote service images do not match the requested bundle")]
    UpgradeRequired,
}
