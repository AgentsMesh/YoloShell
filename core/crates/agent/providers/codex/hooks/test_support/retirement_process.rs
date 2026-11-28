#[path = "admission.rs"]
mod admission_test_support;
#[path = "intake.rs"]
mod intake_test_support;
#[path = "../integration_tests/retirement_fence_process_support.rs"]
mod retirement_fence_process_support;
#[path = "retirement_constants.rs"]
mod retirement_test_constants;
#[path = "retirement.rs"]
mod retirement_test_support;
#[path = "../integration_tests/retirement_fence_process_tests.rs"]
#[cfg(test)]
mod tests;
