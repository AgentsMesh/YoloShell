include!("module_wiring.rs");

#[cfg(any(test, feature = "runtime-component-tests"))]
#[path = "../test_support/fault_injection.rs"]
mod fault_injection;

#[cfg(any(test, feature = "runtime-component-tests"))]
#[path = "../test_support/fault_injection_intake.rs"]
mod fault_injection_intake;

#[cfg(test)]
#[path = "../test_support/identity.rs"]
mod identity;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../test_support/retirement_process.rs"]
#[cfg(test)]
mod retirement_process_tests;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/tests.rs"]
mod tests;
