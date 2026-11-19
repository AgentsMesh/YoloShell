#[cfg(all(test, target_os = "macos"))]
#[path = "secondary_custody_activation_release_race_tests.rs"]
mod activation_release_race_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "secondary_custody_premature_release_tests.rs"]
mod premature_release_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "secondary_custody_registry_failure_tests.rs"]
mod registry_failure_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "secondary_custody_release_commit_tests.rs"]
mod release_commit_tests;
#[cfg(test)]
#[path = "secondary_custody_release_resolution_tests.rs"]
mod release_resolution_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "secondary_custody_retirement_churn_tests.rs"]
mod retirement_churn_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "secondary_custody_retirement_order_tests.rs"]
mod retirement_order_tests;
#[cfg(test)]
#[path = "secondary_custody_transfer_fence_tests.rs"]
mod transfer_fence_tests;
