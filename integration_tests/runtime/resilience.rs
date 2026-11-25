#![cfg(unix)]

include!("resilience/support_imports.rs");

#[path = "fake_agent/constants.rs"]
mod fake_agent_constants;

#[path = "resilience/agent_recovery.rs"]
mod agent_recovery;
#[path = "resilience/child_cleanup.rs"]
mod child_cleanup;
#[path = "resilience/creation_capability.rs"]
mod creation_capability;
#[path = "resilience/creation_cleanup.rs"]
mod creation_cleanup;
#[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
#[path = "resilience/custody_release_recovery.rs"]
mod custody_release_recovery;
#[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
#[path = "resilience/guardian_fail_stop.rs"]
mod guardian_fail_stop;
#[path = "resilience/orphan_guard.rs"]
mod orphan_guard;
#[path = "resilience/process_groups.rs"]
mod process_groups;

#[path = "resilience/support/mod.rs"]
mod support;
use support::*;

#[path = "resilience/hub/mod.rs"]
mod hub;

#[path = "resilience/lifecycle.rs"]
mod lifecycle;
#[path = "resilience/multiplex.rs"]
mod multiplex;
#[path = "resilience/protocol.rs"]
mod protocol;
#[path = "resilience/protocol_ctl.rs"]
mod protocol_ctl;
#[path = "resilience/reboot.rs"]
mod reboot;
#[cfg(feature = "adversarial-test-hooks")]
#[path = "resilience/reconcile_panic.rs"]
mod reconcile_panic;
#[path = "resilience/recovery.rs"]
mod recovery;
#[path = "resilience/relay.rs"]
mod relay;
#[path = "resilience/relay_binary.rs"]
mod relay_binary;
#[path = "resilience/relay_completed.rs"]
mod relay_completed;
#[path = "resilience/relay_lease.rs"]
mod relay_lease;
#[path = "resilience/shell.rs"]
mod shell;
#[path = "resilience/shutdown.rs"]
mod shutdown;
#[path = "resilience/ssh_transport.rs"]
mod ssh_transport;
#[path = "resilience/terminal.rs"]
mod terminal;
#[path = "resilience/terminal_environment/mod.rs"]
mod terminal_environment;
#[cfg(feature = "adversarial-test-hooks")]
#[path = "resilience/termination_recovery.rs"]
mod termination_recovery;
#[path = "resilience/watchdog.rs"]
mod watchdog;
