#[path = "../test_support/child_lifecycle.rs"]
mod child_lifecycle;
mod posix_spawn_child_tests;
mod process_admission_limits;
mod process_cleanup_deadline_tests;
mod process_cleanup_failure_tests;
mod process_descriptor_capability_tests;
mod process_descriptor_cleanup_tests;
mod process_descriptor_mapping_tests;
mod process_environment_tests;
mod process_escape_boundary_tests;
#[path = "../test_support/process_escape_support.rs"]
mod process_escape_support;
mod process_resource_tests;
mod process_supervision_tests;
mod process_tests;
#[path = "../test_support/resources.rs"]
mod resources;
#[path = "../test_support/support.rs"]
mod support;
#[path = "../test_support/test_constants.rs"]
mod test_constants;
