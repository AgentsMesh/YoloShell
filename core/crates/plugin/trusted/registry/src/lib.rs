//! Local trusted-plugin registry. Yolo stores metadata only; the linked source stays in place.

mod digest;
mod lock;
mod paths;
mod record;
mod registry;

pub use digest::source_digest;
pub use paths::RegistryPaths;
pub use record::{RegistryRecord, StartupStatus};
pub use registry::{Registry, RegistryError};
