#[path = "../../codex_hook/hook/admission_tests.rs"]
mod admission_tests;
#[path = "../../codex_hook/exhaustion/exhaustion_fault_tests.rs"]
mod exhaustion_fault_tests;
#[path = "../../codex_hook/exhaustion/exhaustion_handler_tests.rs"]
mod exhaustion_handler_tests;
#[path = "../../codex_hook/exhaustion/exhaustion_runtime_tests.rs"]
mod exhaustion_runtime_tests;
#[path = "../../../test_support/unit/codex_hook/exhaustion/exhaustion_support.rs"]
mod exhaustion_support;
#[path = "../../codex_hook/exhaustion/exhaustion_terminal_tests.rs"]
mod exhaustion_terminal_tests;
#[path = "../../codex_hook/exhaustion/exhaustion_tests.rs"]
mod exhaustion_tests;
#[path = "../../codex_hook/ingress/ingress_accepted_ack_recovery_tests.rs"]
mod ingress_accepted_ack_recovery_tests;
#[path = "../../codex_hook/ingress/ingress_ack_cleanup_adversarial_tests.rs"]
mod ingress_ack_cleanup_adversarial_tests;
#[path = "../../codex_hook/ingress/ingress_ack_persist_recovery_tests.rs"]
mod ingress_ack_persist_recovery_tests;
#[path = "../../codex_hook/ingress/ingress_auto_rebind_adversarial_tests.rs"]
mod ingress_auto_rebind_adversarial_tests;
#[path = "../../codex_hook/ingress/ingress_blocking_custody_tests.rs"]
mod ingress_blocking_custody_tests;
#[path = "../../codex_hook/ingress/ingress_cleanup_sync_recovery_tests.rs"]
mod ingress_cleanup_sync_recovery_tests;
#[path = "../../codex_hook/ingress/ingress_commit_fence_tests.rs"]
mod ingress_commit_fence_tests;
#[path = "../../codex_hook/ingress/ingress_durable_rejection_tests.rs"]
mod ingress_durable_rejection_tests;
#[path = "../../codex_hook/ingress/ingress_generation_fence_tests.rs"]
mod ingress_generation_fence_tests;
#[path = "../../codex_hook/ingress/ingress_intake_settlement_tests.rs"]
mod ingress_intake_settlement_tests;
#[path = "../../codex_hook/ingress/ingress_intake_supervision_tests.rs"]
mod ingress_intake_supervision_tests;
#[path = "../../../test_support/unit/codex_hook/ingress/ingress_live_client.rs"]
mod ingress_live_client;
#[path = "../../codex_hook/ingress/ingress_live_production_tests.rs"]
mod ingress_live_production_tests;
#[path = "../../../test_support/unit/codex_hook/ingress/ingress_live_source.rs"]
mod ingress_live_source;
#[path = "../../codex_hook/ingress/ingress_post_enqueue_failure_tests.rs"]
mod ingress_post_enqueue_failure_tests;
#[path = "../../codex_hook/ingress/ingress_post_rename_commit_tests.rs"]
mod ingress_post_rename_commit_tests;
#[path = "../../codex_hook/ingress/ingress_post_rename_indeterminate_tests.rs"]
mod ingress_post_rename_indeterminate_tests;
#[path = "../../codex_hook/ingress/ingress_rebind_convergence_tests.rs"]
mod ingress_rebind_convergence_tests;
#[path = "../../codex_hook/ingress/ingress_rejection_binding_tests.rs"]
mod ingress_rejection_binding_tests;
#[path = "../../codex_hook/ingress/ingress_state_machine_tests.rs"]
mod ingress_state_machine_tests;
#[path = "../../codex_hook/ingress/ingress_supervisor_tests.rs"]
mod ingress_supervisor_tests;
#[path = "../../codex_hook/ingress/ingress_task_recovery_tests.rs"]
mod ingress_task_recovery_tests;
#[path = "../../../test_support/unit/codex_hook/ingress/ingress_test_assets.rs"]
mod ingress_test_assets;
#[path = "../../../test_support/unit/codex_hook/ingress/ingress_test_support.rs"]
mod ingress_test_support;
#[path = "../../codex_hook/intake/intake_retirement_adversarial_tests.rs"]
mod intake_retirement_adversarial_tests;
#[path = "../../codex_hook/intake/intake_retirement_identity_tests.rs"]
mod intake_retirement_identity_tests;
#[path = "../../codex_hook/intake/intake_retirement_recovery_tests.rs"]
mod intake_retirement_recovery_tests;
#[path = "../../../test_support/unit/codex_hook/intake/intake_retirement_test_support.rs"]
mod intake_retirement_test_support;
#[path = "../../codex_hook/intake/intake_retirement_tests.rs"]
mod intake_retirement_tests;
#[path = "../../codex_hook/intake/intake_task_adversarial_tests.rs"]
mod intake_task_adversarial_tests;
#[path = "../../../test_support/unit/codex_hook/intake/intake_task_test_support.rs"]
mod intake_task_test_support;
#[path = "../../codex_hook/intake/intake_task_tests.rs"]
mod intake_task_tests;
#[path = "../../../test_support/unit/codex_hook/native/native_ingress_support.rs"]
mod native_ingress_support;
#[path = "../../codex_hook/hook/panic_tests.rs"]
mod panic_tests;
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/codex_hook/process_exit_tests.rs"]
mod process_exit_tests;
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../../test_support/integration/codex_hook/process_support.rs"]
mod process_support;
#[path = "../../codex_hook/hook/protocol_tests.rs"]
mod protocol_tests;
#[path = "../../codex_hook/recovery/recovery_adversarial_tests.rs"]
mod recovery_adversarial_tests;
#[path = "../../../test_support/unit/codex_hook/recovery/recovery_fixture_files.rs"]
mod recovery_fixture_files;
#[path = "../../codex_hook/recovery/recovery_install_tests.rs"]
mod recovery_install_tests;
#[path = "../../codex_hook/recovery/recovery_lease_cleanup_tests.rs"]
mod recovery_lease_cleanup_tests;
#[path = "../../../test_support/unit/codex_hook/recovery/recovery_test_support.rs"]
mod recovery_test_support;
#[path = "../../codex_hook/recovery/recovery_tests.rs"]
mod recovery_tests;
#[path = "../../codex_hook/hook/reduction_probe_tests.rs"]
mod reduction_probe_tests;
#[path = "../../codex_hook/registry/registry_fail_closed_tests.rs"]
mod registry_fail_closed_tests;
#[path = "../../codex_hook/registry/registry_key_isolation_tests.rs"]
mod registry_key_isolation_tests;
#[path = "../../codex_hook/registry/registry_takeover_adversarial_tests.rs"]
mod registry_takeover_adversarial_tests;
#[path = "../../codex_hook/registry/registry_takeover_concurrency_tests.rs"]
mod registry_takeover_concurrency_tests;
#[path = "../../codex_hook/registry/registry_takeover_tests.rs"]
mod registry_takeover_tests;
#[path = "../../codex_hook/hook/replay_tests.rs"]
mod replay_tests;
#[path = "../../codex_hook/hook/response_fence_tests.rs"]
mod response_fence_tests;
#[path = "../../codex_hook/hook/routed_protocol_adversarial_tests.rs"]
mod routed_protocol_adversarial_tests;
#[path = "../../codex_hook/runtime/runtime_active_intake_fence_tests.rs"]
mod runtime_active_intake_fence_tests;
#[path = "../../codex_hook/runtime/runtime_fixture_isolation_tests.rs"]
mod runtime_fixture_isolation_tests;
#[path = "../../codex_hook/runtime/runtime_intake_adversarial_tests.rs"]
mod runtime_intake_adversarial_tests;
#[path = "../../codex_hook/runtime/runtime_intake_concurrency_tests.rs"]
mod runtime_intake_concurrency_tests;
#[path = "../../../test_support/unit/codex_hook/runtime/runtime_intake_test_support.rs"]
mod runtime_intake_test_support;
#[path = "../../codex_hook/runtime/runtime_intake_tests.rs"]
mod runtime_intake_tests;
#[path = "../../codex_hook/runtime/runtime_retirement_adversarial_tests.rs"]
mod runtime_retirement_adversarial_tests;
#[path = "../../codex_hook/runtime/runtime_retirement_disposition_tests.rs"]
mod runtime_retirement_disposition_tests;
#[path = "../../codex_hook/runtime/runtime_retirement_intake_adversarial_tests.rs"]
mod runtime_retirement_intake_adversarial_tests;
#[path = "../../../test_support/unit/codex_hook/runtime/runtime_retirement_intake_test_support.rs"]
mod runtime_retirement_intake_test_support;
#[path = "../../codex_hook/runtime/runtime_retirement_intake_tests.rs"]
mod runtime_retirement_intake_tests;
#[path = "../../codex_hook/runtime/runtime_retirement_tests.rs"]
mod runtime_retirement_tests;
#[path = "../../codex_hook/runtime/runtime_tests.rs"]
mod runtime_tests;
#[path = "../../../test_support/unit/codex_hook/server/server_binding_repair_fixture.rs"]
mod server_binding_repair_fixture;
#[path = "../../../test_support/unit/codex_hook/server/server_binding_repair_test_support.rs"]
mod server_binding_repair_test_support;
#[path = "../../codex_hook/server/server_binding_repair_tests.rs"]
mod server_binding_repair_tests;
#[path = "../../codex_hook/server/server_rebind_adversarial_tests.rs"]
mod server_rebind_adversarial_tests;
#[path = "../../codex_hook/server/server_recovery_publication_tests.rs"]
mod server_recovery_publication_tests;
#[path = "../../codex_hook/server/server_registry_tests.rs"]
mod server_registry_tests;
#[path = "../../codex_hook/server/server_route_floor_tests.rs"]
mod server_route_floor_tests;
#[path = "../../codex_hook/server/server_tests.rs"]
mod server_tests;
#[path = "../../codex_hook/hook/snapshot_substitution_tests.rs"]
mod snapshot_substitution_tests;
#[path = "../../codex_hook/hook/stale_tests.rs"]
mod stale_tests;
#[path = "../../codex_hook/hook/storage_failure_tests.rs"]
mod storage_failure_tests;
#[path = "../../codex_hook/hook/substitution_tests.rs"]
mod substitution_tests;
#[path = "../../../test_support/unit/codex_hook/support/support.rs"]
mod support;
#[path = "../../codex_hook/hook/transaction_authority_tests.rs"]
mod transaction_authority_tests;
#[path = "../../../test_support/unit/codex_hook/transport_support.rs"]
mod transport_support;
