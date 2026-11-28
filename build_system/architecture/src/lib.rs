mod checker;
mod constants;
mod source_overlay;
mod sources;
mod workspace;

pub use checker::{inspect, inspect_with_runfiles};
pub use constants::{
    CONFORMANCE_ROOT_ENV, TEST_SOURCE_DIRECTORY_ENV, TEST_WORKSPACE_ENV, WORKSPACE_MARKER,
};
pub use model::Violation;
pub use workspace::{locate, locate_runfiles_overlay};

pub(crate) use yolo_architecture_foundation::model;
pub(crate) use yolo_architecture_target_ownership::inspect_repository as inspect_target_ownership;

#[cfg(test)]
#[path = "../unit_tests/mod.rs"]
mod tests;
