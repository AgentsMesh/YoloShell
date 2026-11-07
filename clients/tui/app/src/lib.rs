pub use run::{Options, run, run_with_shutdown, run_with_startup};
pub use startup::StartupAction;
pub use state::{Mode, State};

include!("module_wiring.rs");

#[cfg(test)]
#[path = "../test_support/client_capabilities.rs"]
mod client_capability_support;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/lib.rs"]
mod integration_tests;
#[cfg(all(test, not(feature = "yolo-integration-tests")))]
#[path = "../unit_tests/lib.rs"]
mod tests;
