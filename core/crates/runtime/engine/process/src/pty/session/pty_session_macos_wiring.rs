#[cfg(target_os = "macos")]
#[path = "proof/pty_session_proof_macos.rs"]
mod macos;
#[cfg(target_os = "macos")]
#[path = "pty_session_candidate_macos.rs"]
mod macos_candidate;
#[cfg(target_os = "macos")]
#[path = "proof/pty_session_proof_contract_macos.rs"]
mod macos_contract;
#[cfg(all(target_os = "macos", any(test, feature = "adversarial-test-hooks")))]
#[path = "pty_session_custody_capture_macos.rs"]
mod macos_custody_capture;
#[cfg(target_os = "macos")]
#[path = "pty_session_custody_scope_macos.rs"]
mod macos_custody_scope;
#[cfg(target_os = "macos")]
#[path = "pty_session_descriptor_authority_macos.rs"]
mod macos_descriptor_authority;
#[cfg(target_os = "macos")]
#[path = "pty_session_descriptor_capability_macos.rs"]
pub(super) mod macos_descriptor_capability;
#[cfg(target_os = "macos")]
#[path = "pty_session_descriptor_scan_macos.rs"]
mod macos_descriptors;
#[cfg(target_os = "macos")]
#[path = "pty_session_exact_descriptor_scan_macos.rs"]
mod macos_exact_descriptor;
#[cfg(target_os = "macos")]
pub(in crate::process) use macos_exact_descriptor::{
    capture_exact_descriptor_identity, has_stable_exact_descriptor,
};
#[cfg(target_os = "macos")]
#[path = "proof/pty_session_proof_fds_macos.rs"]
mod macos_fds;
#[cfg(target_os = "macos")]
#[path = "pty_session_frozen_inventory_macos.rs"]
mod macos_frozen_inventory;
#[cfg(target_os = "macos")]
#[path = "pty_session_global_candidate_macos.rs"]
mod macos_global_candidate;
#[cfg(target_os = "macos")]
#[path = "pty_session_capability_liveness_macos.rs"]
mod macos_liveness;
#[cfg(target_os = "macos")]
#[path = "pty_session_object_identity_macos.rs"]
mod macos_object_identity;
#[cfg(all(feature = "adversarial-test-hooks", not(test), target_os = "macos"))]
#[path = "scan/pty_session_scan_diagnostic_macos.rs"]
pub(in crate::process) mod macos_scan_diagnostic;
#[cfg(target_os = "macos")]
pub(in crate::process) use macos_object_identity::MacosPtyObjectIdentity;
#[cfg(target_os = "macos")]
#[path = "pty_session_object_lease_macos.rs"]
mod macos_object_lease;
#[cfg(target_os = "macos")]
pub(in crate::process) use macos_object_lease::inspect_output_pty_identity;
#[cfg(target_os = "macos")]
#[path = "passive/pty_session_passive_master_macos.rs"]
mod macos_passive_master;
#[cfg(target_os = "macos")]
#[path = "reopen/pty_session_reopen_seal_macos.rs"]
mod macos_reopen_seal;
#[cfg(target_os = "macos")]
#[path = "reopen/pty_session_reopen_seal_master_macos.rs"]
mod macos_reopen_seal_master;
#[cfg(target_os = "macos")]
pub(in crate::process) use macos_reopen_seal::{
    MacosCommittedPtyReopenSeal, MacosFinalizedPtyReopenSeal,
};
#[cfg(target_os = "macos")]
#[path = "pty_session_namespace_loss_reopen_seal_macos.rs"]
mod macos_namespace_loss_reopen_seal;
#[cfg(target_os = "macos")]
#[path = "reopen/pty_session_reopen_seal_io_macos.rs"]
mod macos_reopen_seal_io;
#[cfg(all(any(test, feature = "runtime-component-tests"), target_os = "macos"))]
#[path = "reopen/pty_session_reopen_seal_panic_macos.rs"]
mod macos_reopen_seal_panic;
#[cfg(target_os = "macos")]
#[path = "reopen/pty_session_reopen_seal_state_macos.rs"]
mod macos_reopen_seal_state;
#[cfg(all(
    target_os = "macos",
    any(
        all(test, feature = "yolo-integration-tests"),
        feature = "adversarial-test-hooks"
    )
))]
pub use macos_namespace_loss_reopen_seal::install_pre_finalize_hook;
#[cfg(target_os = "macos")]
pub(in crate::process) use macos_namespace_loss_reopen_seal::{
    MacosNamespaceLossFinalize, MacosNamespaceLossReopenSeal,
};
#[cfg(target_os = "macos")]
pub(crate) use macos_passive_master::MacosPassivePtyMaster;
#[cfg(all(target_os = "macos", any(test, feature = "adversarial-test-hooks")))]
pub(crate) fn corrupt_vnode_identity_for_test(
    identity: MacosPtyObjectIdentity,
    inode_delta: u64,
) -> MacosPtyObjectIdentity {
    MacosPtyObjectIdentity::from_kernel_fields(
        identity.filesystem_device(),
        identity.inode().wrapping_add(inode_delta),
        identity.terminal_rdev(),
    )
}
#[cfg(target_os = "macos")]
pub(in crate::process) use macos_passive_master::MacosPtyHolderExclusions;
#[cfg(all(
    target_os = "macos",
    any(
        all(test, feature = "yolo-integration-tests"),
        feature = "adversarial-test-hooks"
    )
))]
pub use macos_passive_master::install_pending_once as install_namespace_loss_drain_pending_once;
#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/session/custody/pty_session_custody_reopen_seal_macos_tests.rs"]
mod macos_custody_reopen_seal_tests;
#[cfg(target_os = "macos")]
#[path = "pty_session_path_macos.rs"]
mod macos_path;
#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/session/path/pty_session_path_holders_macos_integration_tests.rs"]
mod macos_path_holder_integration_tests;
#[cfg(target_os = "macos")]
#[path = "proof/pty_session_proof_path_holders_macos.rs"]
mod macos_path_holders;
#[cfg(target_os = "macos")]
#[path = "released/pty_session_released_scan_macos.rs"]
mod macos_released;
#[cfg(target_os = "macos")]
#[path = "scan/pty_session_scan_macos.rs"]
mod macos_scan;
#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/session/pty_session_sealed_path_holder_macos_tests.rs"]
mod macos_sealed_path_holder_tests;
#[cfg(target_os = "macos")]
#[path = "pty_session_transferable_candidate_macos.rs"]
mod macos_transferable_candidate;
