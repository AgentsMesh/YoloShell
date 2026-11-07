#[doc(hidden)]
pub mod boot_identity;
#[path = "captured/captured_lineage.rs"]
mod captured_lineage;
#[path = "captured/captured_targets.rs"]
mod captured_targets;
#[path = "captured/captured_tree.rs"]
mod captured_tree;
#[path = "child/child_reaper.rs"]
mod child_reaper;
#[path = "command/command.rs"]
mod command;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[path = "command/command_diagnostic.rs"]
mod command_diagnostic;
#[path = "command/command_private_input.rs"]
pub mod command_private_input;
#[path = "command/command_private_spawn.rs"]
mod command_private_spawn;
#[path = "command/command_settlement.rs"]
mod command_settlement;
#[path = "command/command_worker.rs"]
pub mod command_worker;
mod constants;
mod controller_boundary;
mod descendant_observation;
mod detached_child_custody;
mod direct_child_wait;
#[path = "discovery/discovery_capability.rs"]
mod discovery_capability;
#[path = "discovery/discovery_custody.rs"]
mod discovery_custody;
#[path = "discovery/discovery_custody_budget.rs"]
mod discovery_custody_budget;
#[path = "exact/exact_process_gone.rs"]
mod exact_process_gone;
#[path = "exact/exact_termination.rs"]
mod exact_termination;
#[path = "identity/identity.rs"]
mod identity;
#[path = "identity/identity_relation.rs"]
mod identity_relation;
#[path = "inventory/inventory.rs"]
mod inventory;
#[path = "inventory/inventory_entry.rs"]
mod inventory_entry;
#[path = "inventory/inventory_evidence.rs"]
mod inventory_evidence;
#[path = "inventory/inventory_observation.rs"]
mod inventory_observation;
#[path = "inventory/inventory_scope.rs"]
mod inventory_scope;
#[path = "inventory/inventory_snapshot.rs"]
mod inventory_snapshot;
#[path = "inventory/inventory_stable.rs"]
mod inventory_stable;
#[path = "kernel/kernel_eof_authority.rs"]
pub mod kernel_eof_authority;
#[cfg(target_os = "linux")]
#[path = "linux/linux_containment.rs"]
pub mod linux_containment;
#[cfg(target_os = "linux")]
#[path = "linux/linux_passive_pty_master.rs"]
mod linux_passive_pty_master;
#[cfg(target_os = "linux")]
#[path = "linux/linux_secondary_custody.rs"]
mod linux_secondary_custody;
#[path = "live/live_identity.rs"]
mod live_identity;
#[cfg(target_os = "macos")]
#[path = "macos/macos_kernel.rs"]
mod macos_kernel;
#[cfg(target_os = "macos")]
#[doc(hidden)]
#[path = "macos/macos_native.rs"]
pub mod macos_native;
#[cfg(target_os = "macos")]
#[path = "macos/macos_private_descriptor.rs"]
mod macos_private_descriptor;
#[cfg(target_os = "macos")]
#[path = "macos/macos_process_capability.rs"]
mod macos_process_capability;
#[cfg(target_os = "macos")]
#[path = "macos/macos_process_capability_fd.rs"]
mod macos_process_capability_fd;
#[cfg(target_os = "macos")]
#[path = "macos/macos_secondary_custody.rs"]
mod macos_secondary_custody;
#[cfg(target_os = "macos")]
#[path = "macos/macos_signal.rs"]
pub(crate) mod macos_signal;
mod monotonic;
#[path = "process/process_events.rs"]
mod process_events;
include!("platform_module_wiring.rs");
#[cfg(not(target_os = "macos"))]
pub mod procfs;
#[cfg(all(target_os = "macos", any(test, feature = "adversarial-test-hooks")))]
#[path = "pty/pty_custody_recovery_authority_macos.rs"]
mod pty_custody_recovery_authority_macos;
#[cfg(target_os = "macos")]
#[path = "pty/pty_master_fd_custody_probe_macos.rs"]
mod pty_master_fd_custody_probe_macos;
#[path = "pty/pty_master_release.rs"]
mod pty_master_release;
#[path = "pty/session/proof/pty_session_proof.rs"]
mod pty_session_proof;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[path = "resilience/resilience_adapter.rs"]
mod resilience_adapter;
#[path = "scoped/scoped_waitable_child.rs"]
mod scoped_waitable_child;
mod session_initializer;
#[cfg(unix)]
mod sigchld_waitability;
#[path = "signal/signal.rs"]
mod signal;
#[path = "signal/signal_authority.rs"]
pub mod signal_authority;
#[path = "signal/signal_hook.rs"]
mod signal_hook;
mod unidentified_pty_child_custody;
#[path = "unreaped/unreaped_root_exit.rs"]
mod unreaped_root_exit;
include!("adversarial_test_support_wiring.rs");
