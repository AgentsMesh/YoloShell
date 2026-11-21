//! Public executable image description, never process identity or authority.
//! Capture of a running image belongs to the platform executable-identity layer.
mod constants;
mod error;
mod image;

pub use constants::{BUILD_IMAGE_SCHEMA_VERSION, MAX_EXECUTABLE_IMAGE_BYTES, SHA256_HEX_BYTES};
pub use error::BuildImageError;
pub use image::{BuildImage, BuildImageFields};
