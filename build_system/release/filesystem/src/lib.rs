pub mod constants;

mod acl;
mod directory;
mod directory_lock;
mod directory_resources;
mod temporary_cleanup;
mod temporary_directory;
mod temporary_retirement;
mod tree_manifest;
mod tree_permissions;

pub use acl::ensure_no_extended_acl;
pub use directory::{DirectoryHandle, DirectorySeal};
pub use directory_lock::DirectoryLock;
pub use directory_resources::{DirectoryResourceOwner, DirectoryResourceUse};
pub use temporary_directory::TemporaryDirectory;
pub use tree_manifest::TreeManifest;
pub use tree_permissions::{restore_owner_write, set_read_only};

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/lib.rs"]
mod integration_tests;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
