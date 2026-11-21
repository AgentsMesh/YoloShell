pub use yolo_coordination_core::constants::*;

#[cfg(test)]
pub(super) const TEST_COORDINATION_ROOT_PREFIX: &str = "yct-";
#[cfg(all(test, target_os = "macos"))]
pub(super) const TEST_COORDINATION_PARENT: &str = "/private/tmp";
#[cfg(all(test, not(target_os = "macos")))]
pub(super) const TEST_COORDINATION_PARENT: &str = "/tmp";
