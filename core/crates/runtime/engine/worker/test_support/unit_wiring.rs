#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/codex/codex_hook_ingress_ownership_tests.rs"] mod codex_hook_ingress_ownership_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-lease"))] #[path = "../unit_tests/lease/lease.rs"] mod lease;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/replay.rs"] mod replay;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-attachment"))] #[path = "../unit_tests/attachment/attachment_sequence_exhaustion_tests.rs"] mod attachment_sequence_exhaustion_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-attachment"))] #[path = "../unit_tests/attachment/attachment.rs"] mod attachment;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/exact_termination.rs"] mod exact_termination;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/exited_termination.rs"] mod exited_termination;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/exact_termination_admission.rs"] mod exact_termination_admission;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-dispatch"))] #[path = "../unit_tests/dispatch/dispatch.rs"] mod dispatch;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-terminal"))] #[path = "../unit_tests/terminal/terminal.rs"] mod terminal;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-terminal"))] #[path = "../unit_tests/terminal/terminal_parser_recovery.rs"] mod terminal_parser_recovery;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-terminal"))] #[path = "../unit_tests/terminal/terminal_presentation.rs"] mod terminal_presentation;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/errors.rs"] mod errors;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/connection/connection.rs"] mod connection_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/rpc/rpc_observer.rs"] mod rpc_observer;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/connection/connection_attachment_cursor_tests.rs"] mod connection_attachment_cursor_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/connection/connection_completed_exit_tests.rs"] mod connection_completed_exit_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/runtime/runtime_errors.rs"] mod runtime_errors;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/runtime/runtime_publication_contention_tests.rs"] mod runtime_publication_contention_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/runtime/runtime_activation_publication_tests.rs"] mod runtime_activation_publication_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/native/native_hook_runtime_tests.rs"] mod native_hook_runtime_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/agent/agent_launch_fence_tests.rs"] mod agent_launch_fence_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "unit/agent/agent_launch_fence.rs"] mod agent_launch_fence_support;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "unit/native/native_hook_constants.rs"] mod native_hook_constants;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-state"))] #[path = "../unit_tests/state/state_errors.rs"] mod state_errors;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-journal"))] #[path = "../unit_tests/journal/journal_recovery.rs"] mod journal_recovery;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-journal"))] #[path = "../unit_tests/journal/journal_recovery_matrix.rs"] mod journal_recovery_matrix;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/output/output_delivery.rs"] mod output_delivery;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/agent/agent_monitor.rs"] mod agent_monitor_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/agent/agent_monitor_publication_fence_tests.rs"] mod agent_monitor_publication_fence_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/codex/codex_projection_commit_fence_tests.rs"] mod codex_projection_commit_fence_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/codex/codex_projection_publication_fence_tests.rs"] mod codex_projection_publication_fence_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/codex/codex_projection_resume_boundary_tests.rs"] mod codex_projection_resume_boundary_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/codex/codex_projection_task_finalization_tests.rs"] mod codex_projection_task_finalization_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/agent/agent_projection_shutdown_tests.rs"] mod agent_projection_shutdown_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/agent/agent_projection_shutdown_fence_tests.rs"] mod agent_projection_shutdown_fence_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/runtime/runtime_settlement_error_tests.rs"] mod runtime_settlement_error_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-bootstrap"))] #[path = "../unit_tests/bootstrap/bootstrap_delivery.rs"] mod bootstrap_delivery;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-bootstrap"))] #[path = "../unit_tests/bootstrap/bootstrap_hook_admission.rs"] mod bootstrap_hook_admission;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-bootstrap"))] #[path = "../unit_tests/bootstrap/bootstrap_hook_activation_gate.rs"] mod bootstrap_hook_activation_gate;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-bootstrap"))] #[path = "../unit_tests/bootstrap/bootstrap_hook_inactive_rejections.rs"] mod bootstrap_hook_inactive_rejections;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/input/input_concurrency.rs"] mod input_concurrency;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/input/input_failure_fencing.rs"] mod input_failure_fencing;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/runtime/runtime_epoch_control.rs"] mod runtime_epoch_control;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/runtime/runtime_record_tests.rs"] mod runtime_record_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/snapshot_concurrency.rs"] mod snapshot_concurrency;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/input/input_cache_window.rs"] mod input_cache_window;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/input/input_delivery_capability.rs"] mod input_delivery_capability;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/input/input_delivery_capacity.rs"] mod input_delivery_capacity;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/control/control_barriers.rs"] mod control_barriers;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/secondary_custody/secondary_custody_authority_tests.rs"] mod secondary_custody_authority_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/secondary_custody/secondary_custody_release_tests.rs"] mod secondary_custody_release_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/secondary_custody/secondary_custody_rollback_tests.rs"] mod secondary_custody_rollback_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/secondary_custody/secondary_custody_liveness_adversarial_tests.rs"] mod secondary_custody_liveness_adversarial_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/initial_custody_transfer_fault_tests.rs"] mod initial_custody_transfer_fault_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/secondary_custody/secondary_custody_guardian_runtime_aba_tests.rs"] mod secondary_custody_guardian_runtime_aba_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/secondary_custody/secondary_custody_adoption_adversarial_tests.rs"] mod secondary_custody_adoption_adversarial_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/secondary_custody/secondary_custody_disconnected_adoption_tests.rs"] mod secondary_custody_disconnected_adoption_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/secondary_custody/secondary_custody_committed_reconciliation_tests.rs"] mod secondary_custody_committed_reconciliation_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/secondary_custody/secondary_custody_reconciliation_rejection_tests.rs"] mod secondary_custody_reconciliation_rejection_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/secondary_custody/secondary_custody_export_fence_adversarial_tests.rs"] mod secondary_custody_export_fence_adversarial_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/secondary_custody/secondary_custody_export_failure_adversarial_tests.rs"] mod secondary_custody_export_failure_adversarial_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/secondary_custody/secondary_custody_released_adoption_tests.rs"] mod secondary_custody_released_adoption_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/secondary_custody/secondary_custody_completed_adoption_tests.rs"] mod secondary_custody_completed_adoption_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/secondary_custody/secondary_custody_finalizing_readonly_support.rs"] mod secondary_custody_finalizing_readonly_support;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/secondary_custody/secondary_custody_finalizing_readonly_tests.rs"] mod secondary_custody_finalizing_readonly_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/secondary_custody/secondary_custody_finalizing_readonly_adversarial_tests.rs"] mod secondary_custody_finalizing_readonly_adversarial_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/secondary_custody/secondary_custody_cancellation_tests.rs"] mod secondary_custody_cancellation_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/secondary_custody/secondary_custody_activation_tests.rs"] mod secondary_custody_activation_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/secondary_custody/secondary_custody_activation_failure_tests.rs"] mod secondary_custody_activation_failure_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/secondary_custody/secondary_custody_post_promotion_cancellation_tests.rs"] mod secondary_custody_post_promotion_cancellation_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/secondary_custody/secondary_custody_promotion_reconciliation_tests.rs"] mod secondary_custody_promotion_reconciliation_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/secondary_custody/secondary_custody_promotion_durability_tests.rs"] mod secondary_custody_promotion_durability_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-finalization"))] #[path = "../unit_tests/finalization/finalization_barrier.rs"] mod finalization_barrier;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-finalization"))] #[path = "../unit_tests/finalization/finalization_controller_authority.rs"] mod finalization_controller_authority;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/shutdown_commit_adversarial_tests.rs"] mod shutdown_commit_adversarial_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/response/response_drain.rs"] mod response_drain_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/response/response_drain_connection.rs"] mod response_drain_connection;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/response/response_drain_payloads.rs"] mod response_drain_payloads;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/response/response_drain_runtime.rs"] mod response_drain_runtime;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/response/response_drain_epilogue.rs"] mod response_drain_epilogue;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/response/response_drain_custody.rs"] mod response_drain_custody;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/response/response_drain_service.rs"] mod response_drain_service;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/supervision/supervision_edges.rs"] mod supervision_edges;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/reader/reader_recovery_suite.rs"] mod reader_recovery_suite;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-finalization"))] #[path = "../unit_tests/finalization/finalization_edges.rs"] mod finalization_edges;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-finalization"))] #[path = "../unit_tests/finalization/finalization_fault_drain.rs"] mod finalization_fault_drain;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-finalization"))] #[path = "../unit_tests/finalization/finalization_trace_containment_tests.rs"] mod finalization_trace_containment_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/connection/connection_edges.rs"] mod connection_edges;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/connection/connection_agent_watch.rs"] mod connection_agent_watch;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-protocol"))] #[path = "../unit_tests/protocol/protocol_edges.rs"] mod protocol_edges;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-protocol"))] #[path = "../unit_tests/protocol/protocol_version_admission_tests.rs"] mod protocol_version_admission_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/controller_generation_persistence_tests.rs"] mod controller_generation_persistence_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/auth_identity.rs"] mod auth_identity;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/wire.rs"] mod wire;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/runtime/runtime_task_branches.rs"] mod runtime_task_branches;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/ingress_tests.rs"] mod ingress_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-startup"))] #[path = "../unit_tests/startup/startup_branch_tests.rs"] mod startup_branch_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-startup"))] #[path = "../unit_tests/startup/startup_adversarial.rs"] mod startup_adversarial;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-startup"))] #[path = "../unit_tests/startup/startup_probe_tests.rs"] mod startup_probe_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-startup"))] #[path = "../unit_tests/startup/startup_security_tests.rs"] mod startup_security_tests;
#[cfg(all(test, feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))] #[path = "../unit_tests/launch_cleanup_tests.rs"] mod launch_cleanup_tests;

#[cfg(feature = "yolo-worker-unit-runtime")]
#[path = "../unit_tests/output/output_batch_snapshot.rs"]
mod output_batch_snapshot_tests;

#[cfg(feature = "yolo-worker-unit-runtime")]
#[path = "../unit_tests/output/output_backpressure.rs"]
mod output_backpressure_tests;

#[cfg(feature = "yolo-worker-unit-environment")]
#[path = "../unit_tests/environment/bootstrap.rs"]
mod bootstrap_environment_tests;

#[cfg(feature = "yolo-worker-unit-runtime")]
#[path = "../unit_tests/claude_hook/control_source_tests.rs"]
mod native_claude_control_source_tests;

#[cfg(feature = "yolo-worker-unit-native_control")]
#[path = "../unit_tests/native_control/mod.rs"]
mod native_control_tests;
