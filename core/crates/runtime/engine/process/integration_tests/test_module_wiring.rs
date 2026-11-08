#[cfg(all(test, target_os = "macos"))]
#[path = "captured/captured_lineage_tests.rs"]
mod captured_lineage_tests;
#[cfg(test)]
#[path = "captured/tree/captured_tree_authority_loss_tests.rs"]
mod captured_tree_authority_loss_tests;
#[cfg(test)]
#[path = "captured/tree/captured_tree_bootstrap_failure_tests.rs"]
mod captured_tree_bootstrap_failure_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "captured/tree/captured_tree_boundary_tests.rs"]
mod captured_tree_boundary_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "captured/tree/captured_tree_compaction_tests.rs"]
mod captured_tree_compaction_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "captured/tree/discovery/captured_tree_discovery_adversarial_tests.rs"]
mod captured_tree_discovery_adversarial_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "captured/tree/discovery/captured_tree_discovery_test_support.rs"]
mod captured_tree_discovery_test_support;
#[cfg(test)]
#[path = "captured/tree/captured_tree_dynamic_tests.rs"]
mod captured_tree_dynamic_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "captured/tree/captured_tree_event_tests.rs"]
mod captured_tree_event_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "captured/tree/captured_tree_lineage_tests.rs"]
mod captured_tree_lineage_tests;
#[cfg(all(test, target_os = "linux"))]
#[path = "captured/tree/captured_tree_linux_authority_tests.rs"]
mod captured_tree_linux_authority_tests;
#[cfg(all(test, target_os = "linux"))]
#[path = "captured/tree/captured_tree_linux_lifecycle_tests.rs"]
mod captured_tree_linux_lifecycle_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "captured/tree/macos/captured_tree_macos_adversarial_tests.rs"]
mod captured_tree_macos_adversarial_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "captured/tree/captured_tree_spawn_boundary_tests.rs"]
mod captured_tree_spawn_boundary_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "captured/tree/captured_tree_survivor_discovery_macos_tests.rs"]
mod captured_tree_survivor_discovery_macos_tests;
#[cfg(test)]
#[path = "captured/tree/captured_tree_survivor_tests.rs"]
mod captured_tree_survivor_tests;
#[cfg(test)]
#[path = "captured/tree/captured_tree_tests.rs"]
mod captured_tree_tests;
#[cfg(test)]
#[path = "child/child_reaper_tests.rs"]
mod child_reaper_tests;
#[cfg(test)]
#[path = "command/command_private_input_tests.rs"]
mod command_private_input_tests;
#[cfg(test)]
#[path = "command/command_deadline_tests.rs"]
mod command_deadline_tests;
#[cfg(test)]
#[path = "command/command_settlement_tests.rs"]
mod command_settlement_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "discovery/discovery_custody_tests.rs"]
mod discovery_custody_tests;
#[cfg(test)]
#[path = "inventory/inventory_error_tests.rs"]
mod inventory_error_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "inventory/inventory_live_stability_tests.rs"]
mod inventory_live_stability_tests;
#[cfg(test)]
#[path = "inventory/inventory_stability_tests.rs"]
mod inventory_stability_tests;
#[cfg(test)]
#[path = "inventory/inventory_tests.rs"]
mod inventory_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "live/live_identity_tests.rs"]
mod live_identity_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "pty/session/descriptor/pty_session_descriptor_capability_reuse_tests.rs"]
mod pty_session_descriptor_capability_reuse_tests;
#[cfg(all(test, target_os = "linux"))]
#[path = "pty/session/linux/pty_session_linux_integration_tests.rs"]
mod pty_session_linux_integration_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "resilience/resilience_adapter_double_fork_tests.rs"]
mod resilience_adapter_double_fork_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "resilience/resilience_adapter_drop_containment_tests.rs"]
mod resilience_adapter_drop_containment_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "resilience/resilience_adapter_drop_persistent_failure_tests.rs"]
mod resilience_adapter_drop_persistent_failure_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "resilience/resilience_adapter_force_tests.rs"]
mod resilience_adapter_force_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "resilience/resilience_adapter_handoff_tests.rs"]
mod resilience_adapter_handoff_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "resilience/resilience_adapter_reopen_seal_fence_tests.rs"]
mod resilience_adapter_reopen_seal_fence_tests;
#[cfg(all(test, target_os = "macos"))]
#[path = "resilience/resilience_adapter_test_support.rs"]
mod resilience_adapter_test_support;
#[cfg(test)]
#[path = "resilience/resilience_adapter_tests.rs"]
mod resilience_adapter_tests;
#[cfg(test)]
#[path = "resilience/resilience_cleanup_containment_test_support.rs"]
mod resilience_cleanup_containment_test_support;
#[cfg(all(test, target_os = "macos"))]
#[path = "retained/retained_exact_double_fork_support.rs"]
mod retained_exact_double_fork_support;
#[cfg(all(test, target_os = "macos"))]
#[path = "retained/retained_exact_double_fork_tests.rs"]
mod retained_exact_double_fork_tests;
#[cfg(test)]
#[path = "signal/signal_fence_tests.rs"]
mod signal_fence_tests;
#[cfg(test)]
mod tests;
