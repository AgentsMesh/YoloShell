pub mod constants;

mod posix_cleanup;
mod posix_spawn;
mod posix_spawn_actions;
mod posix_spawn_descriptors;
mod posix_spawn_strings;
mod posix_supervisor;
mod process;
mod process_resources;
mod process_descriptor;
mod process_group;
mod process_group_members;
mod process_spawn_lock;
mod process_wait;

pub use process::ProcessCommand;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../test_support/unit_support.rs"]
mod unit_support;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/lib.rs"]
mod integration_tests;
