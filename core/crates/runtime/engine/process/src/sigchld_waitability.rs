pub use yolo_process_spawn::{
    DescriptorScanGuard, WaitableChildSpawnBoundary, try_descriptor_scan_guard,
    try_waitable_child_spawn_guard, waitable_child_spawn_guard_before,
};
pub use yolo_process_spawn::{WaitableChildSpawnGuard, waitable_child_spawn_guard};

#[cfg(all(test, feature = "yolo-integration-tests"))]
pub(crate) use yolo_process_spawn::reject_spawn_gate_entry_for_test;

#[cfg(test)]
#[cfg(feature = "yolo-integration-tests")]
use yolo_process_spawn::{normalize_sigchld_waitability, read_sigchld_action, set_sigchld_action};

#[cfg(test)]
#[cfg(feature = "yolo-integration-tests")]
pub(crate) mod test_hook {
    pub use yolo_process_spawn::test_hook::*;
}

#[cfg(test)]
#[cfg(feature = "yolo-integration-tests")]
#[path = "../integration_tests/sigchld_waitability_tests.rs"]
mod tests;
