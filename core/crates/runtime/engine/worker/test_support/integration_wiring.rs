#[cfg(all(test, feature = "yolo-integration-tests"))] #[path = "../integration_tests/runtime/runtime_exec_report_adversarial.rs"] mod runtime_exec_report_adversarial;
#[cfg(all(test, feature = "yolo-integration-tests"))] #[path = "../integration_tests/pty/pty_edges.rs"] mod pty_edges;
#[cfg(all(test, feature = "yolo-integration-tests"))] #[path = "../integration_tests/pty/pty_eof_supervision.rs"] mod pty_eof_supervision;
#[cfg(all(test, feature = "yolo-integration-tests"))] #[path = "../integration_tests/pty/pty_native_close.rs"] mod pty_native_close;
#[cfg(all(test, feature = "yolo-integration-tests"))] #[path = "../integration_tests/pty/pty_panic_supervision.rs"] mod pty_panic_supervision;
#[cfg(all(test, feature = "yolo-integration-tests"))] #[path = "../integration_tests/pty/supervisor/pty_supervisor_matrix.rs"] mod pty_supervisor_matrix;
#[cfg(all(test, feature = "yolo-integration-tests"))] #[path = "../integration_tests/runtime/runtime_activation_order.rs"] mod runtime_activation_order;
#[cfg(all(test, feature = "yolo-integration-tests"))] #[path = "../integration_tests/pty/pty_control_barriers.rs"] mod pty_control_barriers;
#[cfg(all(test, feature = "yolo-integration-tests"))] #[path = "../integration_tests/pty/pty_termination_disconnect_tests.rs"] mod pty_termination_disconnect_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))] #[path = "../integration_tests/pty/runtime/pty_runtime_alias_custody_tests.rs"] mod pty_runtime_alias_custody_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))] #[path = "../integration_tests/pty/runtime/pty_runtime_alias_authority_tests.rs"] mod pty_runtime_alias_authority_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))] #[path = "../integration_tests/pty/runtime/pty_runtime_alias_release_tests.rs"] mod pty_runtime_alias_release_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))] #[path = "../integration_tests/pty/runtime/pty_runtime_alias_recovery_tests.rs"] mod pty_runtime_alias_recovery_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))] #[path = "../integration_tests/pty/runtime/pty_runtime_alias_release_retry_tests.rs"] mod pty_runtime_alias_release_retry_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))] #[path = "../integration_tests/pty/pty_input_starvation.rs"] mod pty_input_starvation;
#[cfg(all(test, feature = "yolo-integration-tests"))] #[path = "../integration_tests/pty/missing/pty_missing_eof_finalization_tests.rs"] mod pty_missing_eof_finalization_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))] #[path = "../integration_tests/runtime/runtime_exit_process_support.rs"] mod runtime_exit_process_support;
#[cfg(all(test, feature = "yolo-integration-tests"))] #[path = "integration/runtime_error/runtime_error_process_watchdog.rs"] mod runtime_error_process_watchdog;
#[cfg(all(test, feature = "yolo-integration-tests"))] #[path = "../integration_tests/runtime/runtime_error_process_watchdog_adversarial.rs"] mod runtime_error_process_watchdog_adversarial;
#[cfg(all(test, feature = "yolo-integration-tests"))] #[path = "../integration_tests/real/real_pty_exit.rs"] mod real_pty_exit;
#[cfg(all(test, feature = "yolo-integration-tests"))] #[path = "../integration_tests/pty/owner/pty_owner_cancellation.rs"] mod pty_owner_cancellation;
#[cfg(all(test, feature = "yolo-integration-tests"))] #[path = "../integration_tests/pty/owner/pty_owner_fault.rs"] mod pty_owner_fault;
#[cfg(all(test, feature = "yolo-integration-tests"))] #[path = "../integration_tests/pty/owner/pty_owner_extraction_recovery.rs"] mod pty_owner_extraction_recovery;
#[cfg(all(test, feature = "yolo-integration-tests"))] #[path = "../integration_tests/pty/owner/pty_owner_error_containment_tests.rs"] mod pty_owner_error_containment_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))] #[path = "../integration_tests/pty/owner/pty_owner_tracking_fault.rs"] mod pty_owner_tracking_fault;
#[cfg(all(test, feature = "yolo-integration-tests"))] #[path = "../integration_tests/pty/owner/pty_owner_wait_fault.rs"] mod pty_owner_wait_fault;
#[cfg(all(test, feature = "yolo-integration-tests"))] #[path = "../integration_tests/startup/startup_bootstrap.rs"] mod startup_bootstrap;

#[path = "../integration_tests/native_control/mod.rs"]
mod native_control_tests;

#[path = "../integration_tests/bash/bash_locale.rs"]
mod bash_locale_tests;

#[path = "../integration_tests/bash/bash_fixture_cleanup.rs"]
mod bash_fixture_cleanup_tests;

#[path = "../integration_tests/guardian/guardian_runtime_directory.rs"]
mod guardian_runtime_directory_tests;
