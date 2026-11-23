mod authority_validation_matrix_tests;
mod authority_lifecycle_matrix_tests;
mod begin_identity_matrix_tests;
#[path = "binding/binding_transition_tests.rs"]
mod binding_transition_tests;
#[path = "binding/binding_validation_matrix_tests.rs"]
mod binding_validation_matrix_tests;
#[path = "capacity_adversarial_tests.rs"]
mod capacity_adversarial_tests;
#[path = "durable/durable_publication_owner_fence_tests.rs"]
mod durable_publication_owner_fence_tests;
#[path = "durable/durable_state_adversarial_tests.rs"]
mod durable_state_adversarial_tests;
#[path = "durable/durable_state_fence_tests.rs"]
mod durable_state_fence_tests;
#[path = "durable/durable_state_publication_tests.rs"]
mod durable_state_publication_tests;
#[path = "durable/durable_state_validation_tests.rs"]
mod durable_state_validation_tests;
mod exit_binding_matrix_tests;
#[path = "foreground_reactivation_durable_tests.rs"]
mod foreground_reactivation_durable_tests;
#[path = "hook_loss_tests.rs"]
mod hook_loss_tests;
#[path = "../../test_support/generation/hook_publication_fence_test_support.rs"]
mod hook_publication_fence_test_support;
#[path = "hook_publication_fence_tests.rs"]
mod hook_publication_fence_tests;
#[path = "hook_scan_fence_tests.rs"]
mod hook_scan_fence_tests;
#[path = "invocation_set_tests.rs"]
mod invocation_set_tests;
mod invocation_set_validation_matrix_tests;
#[path = "managed/managed_manifest_fence_tests.rs"]
mod managed_manifest_fence_tests;
#[path = "../../test_support/generation/managed_mutation_support.rs"]
mod managed_mutation_support;
#[path = "managed/managed_mutation_tests.rs"]
mod managed_mutation_tests;
#[path = "managed/managed_shim_exec_durable_tests.rs"]
mod managed_shim_exec_durable_tests;
#[path = "managed/managed_shim_exec_fence_tests.rs"]
mod managed_shim_exec_fence_tests;
#[path = "../../test_support/generation/managed_shim_exec_publication_fixture.rs"]
mod managed_shim_exec_publication_fixture;
#[path = "../../test_support/generation/managed_shim_exec_publication_test_support.rs"]
mod managed_shim_exec_publication_test_support;
#[path = "managed/managed_shim_exec_publication_tests.rs"]
mod managed_shim_exec_publication_tests;
#[path = "monitor_race_tests.rs"]
mod monitor_race_tests;
#[path = "../../test_support/generation/observed_unmanaged_publication_test_support.rs"]
mod observed_unmanaged_publication_test_support;
#[path = "observed_unmanaged_publication_tests.rs"]
mod observed_unmanaged_publication_tests;
#[path = "participant_wait_adversarial_tests.rs"]
mod participant_wait_adversarial_tests;
#[path = "../../test_support/generation/pending_predecessor_adversarial_test_support.rs"]
mod pending_predecessor_adversarial_test_support;
#[path = "pending_predecessor_adversarial_tests.rs"]
mod pending_predecessor_adversarial_tests;
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/generation/process_exit_tests.rs"]
mod process_exit_tests;
#[path = "published_authority_snapshot_tests.rs"]
mod published_authority_snapshot_tests;
#[path = "reboot_validation_tests.rs"]
mod reboot_validation_tests;
#[path = "../../test_support/generation/recovery_event_support.rs"]
mod recovery_event_support;
#[path = "recovery/recovery_evidence_adversarial_tests.rs"]
mod recovery_evidence_adversarial_tests;
#[path = "recovery/recovery_identity_tests.rs"]
mod recovery_identity_tests;
#[path = "../../test_support/generation/recovery_orphan_test_support.rs"]
mod recovery_orphan_test_support;
#[path = "recovery/recovery_orphan_tests.rs"]
mod recovery_orphan_tests;
#[path = "binding/binding_predecessor_retirement_tests.rs"]
mod binding_predecessor_retirement_tests;
#[path = "recovery/recovery_publication_tests.rs"]
mod recovery_publication_tests;
#[path = "../../test_support/generation/recovery_test_support.rs"]
mod recovery_test_support;
#[path = "recovery/recovery_transaction_tests.rs"]
mod recovery_transaction_tests;
#[path = "schema_compatibility_tests.rs"]
mod schema_compatibility_tests;
#[path = "security_tests.rs"]
mod security_tests;
#[path = "session_end_tests.rs"]
mod session_end_tests;
#[path = "session_restart_tests.rs"]
mod session_restart_tests;
#[path = "session_start_durable_transaction_tests.rs"]
mod session_start_durable_transaction_tests;
#[path = "stale_writer_adversarial_tests.rs"]
mod stale_writer_adversarial_tests;
#[path = "store_tests.rs"]
mod store_tests;
#[path = "../../test_support/generation/support.rs"]
mod support;
#[path = "transition_exit_fence_tests.rs"]
mod transition_exit_fence_tests;
#[path = "transition_tests.rs"]
mod transition_tests;

#[path = "../../test_support/generation/binding_transition_contract.rs"]
mod binding_transition_contract;
#[path = "binding/binding_transition_contract_tests.rs"]
mod binding_transition_contract_tests;
#[path = "binding/binding_transition_source_tests.rs"]
mod binding_transition_source_tests;
