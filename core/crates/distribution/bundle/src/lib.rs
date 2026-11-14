//! Validated identities for releases containing one executable and explicit process roles.
mod artifact;
mod binary_format;
mod constants;
mod descriptor;
mod error;
mod identity;
mod role;
mod source;
mod target;
mod validation;

pub use artifact::{ArtifactDescriptor, ArtifactFields, BundleArtifacts};
pub use binary_format::binary_target;
pub use constants::{
    BIN_DIRECTORY, BUNDLE_SCHEMA_VERSION, MANIFEST_FILE_NAME, MAX_ARTIFACT_BYTES,
    MAX_VERSION_BYTES, RELEASES_DIRECTORY, SHA256_HEX_BYTES, SOURCE_REVISION_HEX_BYTES,
};
pub use descriptor::{BundleDescriptor, BundleFields};
pub use error::BundleError;
pub use role::ArtifactRole;
pub use source::BundleSource;
pub use target::{Architecture, Platform, Target};
