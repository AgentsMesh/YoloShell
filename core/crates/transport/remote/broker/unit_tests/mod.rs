#[path = "bootstrap/bootstrap_failure_policy_tests.rs"]
mod bootstrap_failure_policy_tests;
#[path = "../test_support/runtime_client.rs"]
mod runtime_client;
#[path = "../test_support/runtime_shutdown.rs"]
mod runtime_shutdown;
#[path = "bootstrap/bootstrap_liveness_tests.rs"]
mod bootstrap_liveness_tests;
#[path = "branch_convergence_tests.rs"]
mod branch_convergence_tests;
#[path = "broker/broker_namespace_tests.rs"]
mod broker_namespace_tests;
#[path = "broker/broker_paths_tests.rs"]
mod broker_paths_tests;
#[path = "cleanup/cleanup_registry_adversarial_tests.rs"]
mod cleanup_registry_adversarial_tests;
#[path = "config/config_authority_tests.rs"]
mod config_authority_tests;
#[path = "config/config_edge_tests.rs"]
mod config_edge_tests;
#[path = "config/config_expected_tests.rs"]
mod config_expected_tests;
#[path = "config/config_passphrase_tests.rs"]
mod config_passphrase_tests;
#[path = "config/config_revision_tests.rs"]
mod config_revision_tests;
#[path = "config/config_tests.rs"]
mod config_tests;
#[path = "factory/factory_failure_tests.rs"]
mod factory_failure_tests;
#[path = "factory/factory_tests.rs"]
mod factory_tests;
#[path = "forwarding_adversarial_tests.rs"]
mod forwarding_adversarial_tests;
#[path = "handler_tests.rs"]
mod handler_tests;
#[path = "handoff/handoff_transition_tests.rs"]
mod handoff_transition_tests;
#[path = "hub/hub_config_tests.rs"]
mod hub_config_tests;
#[path = "hub/hub_mode_tests.rs"]
mod hub_mode_tests;
#[path = "launch/launch_authority_tests.rs"]
mod launch_authority_tests;
#[path = "launch/launch_publication_tests.rs"]
mod launch_publication_tests;
#[path = "management/management_command_cancellation_tests.rs"]
mod management_command_cancellation_tests;
#[cfg(target_os = "macos")]
#[path = "management/management_transport_tests.rs"]
mod management_transport_tests;
#[path = "operations/operations_adversarial_tests.rs"]
mod operations_adversarial_tests;
#[path = "operations/operations_tests.rs"]
mod operations_tests;
#[path = "pool/pool_adversarial_tests.rs"]
mod pool_adversarial_tests;
#[path = "pool/pool_tests.rs"]
mod pool_tests;
#[path = "protocol/protocol_tests.rs"]
mod protocol_tests;
#[path = "readiness_policy_tests.rs"]
mod readiness_policy_tests;
#[path = "rotation_fence_tests.rs"]
mod rotation_fence_tests;
#[path = "runtime/runtime_bootstrap_deadline_tests.rs"]
mod runtime_bootstrap_deadline_tests;
#[path = "runtime/runtime_forward_shutdown_error_tests.rs"]
mod runtime_forward_shutdown_error_tests;
#[path = "runtime/runtime_forward_tests.rs"]
mod runtime_forward_tests;
#[path = "runtime/runtime_generation_tests.rs"]
mod runtime_generation_tests;
#[path = "runtime/runtime_handshake_tests.rs"]
mod runtime_handshake_tests;
#[path = "runtime/runtime_open_deadline_tests.rs"]
mod runtime_open_deadline_tests;
#[path = "runtime/runtime_publication_deadline_tests.rs"]
mod runtime_publication_deadline_tests;
#[cfg(target_os = "macos")]
#[path = "runtime/runtime_transport_tests.rs"]
mod runtime_transport_tests;
#[path = "security/security_tests.rs"]
mod security_tests;
#[path = "server/server_accept_tests.rs"]
mod server_accept_tests;
#[path = "server/server_adversarial_tests.rs"]
mod server_adversarial_tests;
#[path = "server/server_entrypoint_tests.rs"]
mod server_entrypoint_tests;
#[path = "server/server_maintenance_shutdown_tests.rs"]
mod server_maintenance_shutdown_tests;
#[path = "server/server_retirement_tests.rs"]
mod server_retirement_tests;
#[path = "server/server_shutdown_tests.rs"]
mod server_shutdown_tests;
#[path = "server/server_support.rs"]
mod server_support;
#[path = "server/server_tests.rs"]
mod server_tests;
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/sockets_hook_tests.rs"]
mod sockets_hook_tests;
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/sockets_tests.rs"]
mod sockets_tests;
#[path = "support/support.rs"]
mod support;
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/support_channel.rs"]
mod support_channel;
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/support_channel_probe.rs"]
mod support_channel_probe;
#[path = "support/support_connection.rs"]
mod support_connection;
#[path = "support/support_operations.rs"]
mod support_operations;
#[path = "provisioning/provisioning_admission.rs"]
mod provisioning_admission;
#[path = "provisioning/provisioning_client.rs"]
mod provisioning_client;
