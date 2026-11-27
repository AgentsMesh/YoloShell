use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum BundleError {
    #[error("bundle source path is not bounded and canonical")]
    SourcePath,
    #[error("unsupported bundle schema")]
    Schema,
    #[error("invalid source revision")]
    SourceRevision,
    #[error("invalid bundle version")]
    Version,
    #[error("invalid artifact digest")]
    Digest,
    #[error("artifact size is outside the bundle limit")]
    Size,
    #[error("bundle build identity does not match its contents")]
    BuildIdentity,
    #[error("artifact bytes do not match their descriptor")]
    ArtifactIntegrity,
    #[error("artifact binary target does not match its bundle")]
    ArtifactTarget,
}
