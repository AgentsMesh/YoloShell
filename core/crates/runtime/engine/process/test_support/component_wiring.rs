#[cfg(target_os = "macos")]
#[path = "captured_tree/captured_tree_macos_pty_fixture.rs"]
mod captured_tree_macos_pty_fixture;
#[cfg(target_os = "macos")]
#[path = "captured_tree/captured_tree_macos_support.rs"]
mod captured_tree_macos_support;
#[cfg(target_os = "macos")]
#[path = "captured_tree/captured_tree_macos_wait.rs"]
mod captured_tree_macos_wait;
#[path = "inventory/inventory_test_support.rs"]
mod inventory_test_support;
#[cfg(target_os = "macos")]
#[path = "runtime/runtime_macos_api.rs"]
mod runtime_macos_api;
#[path = "runtime/runtime_tree_api.rs"]
mod runtime_tree_api;
#[path = "waitable_child/waitable_child_test_support.rs"]
mod waitable_child_test_support;
#[cfg(target_os = "macos")]
pub use runtime_macos_api::{
    KernelProcessMetadataForTest, kernel_process_metadata_for_test, with_test_signal_capability,
};

#[cfg(target_os = "macos")]
pub use captured_targets::test_hook::install_test_signal_hook;
pub use captured_tree::CapturedProcessTreeStateSnapshot;
#[cfg(target_os = "macos")]
pub use captured_tree_macos_pty_fixture::{LineageFixture, PtyLineageFixture};
#[cfg(target_os = "macos")]
pub use captured_tree_macos_support::run_lineage_helper;
pub use live_identity::install_test_match_failure;
pub use waitable_child_test_support::spawn_waitable_test_child_in_boundary;
