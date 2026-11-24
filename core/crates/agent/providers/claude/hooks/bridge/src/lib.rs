pub use yolo_claude_hooks as claude_hooks;
use yolo_local_unix as local_unix;
mod constants;
mod hooks;
mod native_client;
mod native_delivery;
mod native_parent;
mod permission;
pub mod timing;

pub use hooks::run;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../test_support/native_parent.rs"]
mod parent_test_support;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../test_support/constants.rs"]
mod test_constants;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../test_support/native_delivery.rs"]
mod delivery_test_support;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/permission_tests.rs"]
mod permission_tests;
