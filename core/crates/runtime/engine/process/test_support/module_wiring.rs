#[cfg(target_os = "macos")]
#[path = "captured_tree/captured_tree_macos_pty_fixture.rs"]
mod captured_tree_macos_pty_fixture;
#[cfg(target_os = "macos")]
#[path = "captured_tree/captured_tree_macos_support.rs"]
pub(crate) mod captured_tree_macos_support;
#[cfg(target_os = "macos")]
#[path = "captured_tree/captured_tree_macos_wait.rs"]
mod captured_tree_macos_wait;
#[path = "command_settlement_test_hook.rs"]
mod command_settlement_test_hook;
#[cfg(all(target_os = "macos", feature = "yolo-integration-tests"))]
#[path = "descriptor_transfer.rs"]
pub(crate) mod descriptor_transfer_test_support;
#[path = "identity_relation_test_hook.rs"]
mod identity_relation_test_hook;
#[path = "inventory/inventory_test_support.rs"]
pub(crate) mod inventory_test_support;
#[cfg(feature = "yolo-integration-tests")]
#[path = "pty_spawn.rs"]
pub(crate) mod pty_spawn_test_support;
#[path = "waitable_child/waitable_child_test_support.rs"]
mod waitable_child_test_support;

pub(crate) use captured_tree::CapturedProcessTreeStateSnapshot;
pub(crate) use live_identity::{
    install_test_live_execution_state_failure, install_test_live_signal_failure,
    install_test_live_signal_observer, install_test_match_failure,
};
pub(crate) use waitable_child_test_support::{
    reaped_test_process_identity, spawn_waitable_test_child, spawn_waitable_test_child_in_boundary,
};

pub(crate) fn install_repeating_process_inventory_for_test(
    entries: &[(ProcessIdentity, u32, u32)],
) -> impl Drop + use<> {
    let snapshot = entries
        .iter()
        .map(|(identity, parent_pid, session_id)| {
            inventory_test_support::entry(*identity, *parent_pid, *session_id)
        })
        .collect();
    inventory_snapshot::install_repeating_test_inventory(snapshot)
}
