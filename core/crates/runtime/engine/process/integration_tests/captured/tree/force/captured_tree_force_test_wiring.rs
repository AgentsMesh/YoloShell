use super::transition::ProcessTransitionError;

#[cfg(test)]
#[path = "captured_tree_force_basis_matrix_tests.rs"]
mod basis_matrix_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "captured_tree_force_empty_authority_error_tests.rs"]
mod empty_authority_error_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "../captured_tree_empty_eof_authority_tests.rs"]
mod empty_eof_authority_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "../captured_tree_empty_eof_fixture.rs"]
mod empty_eof_fixture;
#[cfg(test)]
#[path = "captured_tree_force_empty_fence_tests.rs"]
mod empty_fence_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "captured_tree_force_empty_reopen_seal_tests.rs"]
mod empty_reopen_seal_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "captured_tree_force_empty_token_tests.rs"]
mod empty_token_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "captured_tree_force_exit_authority_tests.rs"]
mod exit_authority_tests;
#[cfg(test)]
#[path = "../gone/captured_tree_gone_confirmation_tests.rs"]
mod gone_confirmation_tests;
#[cfg(test)]
#[path = "../gone/captured_tree_gone_report_tests.rs"]
mod gone_report_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "captured_tree_force_race_macos_tests.rs"]
mod macos_race_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "captured_tree_force_macos_tests.rs"]
mod macos_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "captured_tree_force_no_proof_macos_tests.rs"]
mod no_proof_macos_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "../../../pty/session/reopen/pty_session_reopen_seal_lifecycle_macos_tests.rs"]
mod reopen_seal_lifecycle_macos_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "../../../pty/session/reopen/pty_session_reopen_seal_natural_retirement_macos_tests.rs"]
mod reopen_seal_natural_retirement_macos_tests;
#[cfg(test)]
#[path = "../captured_tree_reported_retirement_authority_tests.rs"]
mod reported_retirement_authority_tests;
#[cfg(test)]
#[path = "../captured_tree_retirement_authority_tests.rs"]
mod retirement_authority_tests;
#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
#[path = "../root/captured_tree_root_exit_gone_tests.rs"]
mod root_exit_gone_tests;
#[cfg(test)]
#[path = "captured_tree_force_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "captured_tree_force_version_tests.rs"]
mod version_tests;
