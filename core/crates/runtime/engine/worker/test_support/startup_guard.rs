#[cfg(target_os = "macos")]
#[path = "integration/startup_guard/startup_guard_process_descriptor.rs"]
mod process_descriptor;
#[path = "integration/startup_guard/startup_guard_process_fixture.rs"]
mod process_fixture;
#[path = "integration/startup_guard/startup_guard_production_fixture.rs"]
pub(super) mod production_fixture;

pub(super) use process_fixture::{
    TEST_MARKER_FILE, spawn_test_guard, wait_for_descendant, wait_for_tracked_identity,
};
