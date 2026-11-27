#[cfg(test)]
include!("../test_support/module_wiring.rs");
#[cfg(all(not(test), feature = "runtime-component-tests"))]
include!("../test_support/component_wiring.rs");
#[cfg(all(test, feature = "yolo-integration-tests"))]
include!("../integration_tests/test_module_wiring.rs");
