use super::{
    ProcessIdentity, captured_lineage::ObservedProcess, inventory_stable::StableInventory,
};
use anyhow::Result;
use std::collections::HashSet;
use std::path::PathBuf;
pub struct PtySessionProof {
    #[cfg(target_os = "macos")]
    tty_path: PathBuf,
    #[cfg(target_os = "linux")]
    _tty_path: PathBuf,
    #[cfg(target_os = "linux")]
    tty_device: u64,
    #[cfg(target_os = "macos")]
    tty_object_identity: Option<macos_object_identity::MacosPtyObjectIdentity>,
    #[cfg(target_os = "macos")]
    pty_object_lease: Option<macos_object_lease::MacosPtyObjectLease>,
    #[cfg(target_os = "macos")]
    reopen_seal: macos_reopen_seal_state::MacosPtyReopenSealState,
    #[cfg(all(target_os = "macos", any(test, feature = "adversarial-test-hooks")))]
    custody_probe: Option<super::PtyMasterFdCustodyProbe>,
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    _tty_path: PathBuf,
    capability: String,
    runtime: Option<super::PtyRuntimeIdentity>,
    master_alias_state: PtyMasterAliasState,
    scope: CapabilityScope,
    bootstrap: bootstrap_fence::BootstrapFenceState,
    bootstrap_exec: bootstrap_exec::BootstrapExecState,
    #[cfg(target_os = "macos")]
    descriptor_capability: Option<macos_descriptor_authority::MacosDescriptorAuthority>,
}
#[cfg(target_os = "linux")]
#[path = "../linux/pty_session_linux_origin.rs"]
mod linux_origin;

#[cfg(any(
    all(test, feature = "yolo-test-support"),
    all(not(test), feature = "adversarial-test-hooks")
))]
include!("../../../../test_support/pty_session/pty_session_proof.rs");
#[derive(Clone, Copy, Eq, PartialEq)]
enum CapabilityScope {
    Pty,
    ProcessDiscovery,
    CustodyPtyObject,
}

#[derive(Clone, Default)]
pub(super) struct PtyCapabilityScanState {
    #[cfg(target_os = "macos")]
    released: macos_released::ReleasedScanState,
    #[cfg(target_os = "macos")]
    liveness: macos_liveness::MacosCapabilityLivenessState,
    #[cfg(target_os = "linux")]
    linux: linux_scan_state::LinuxScanState,
}
#[path = "../scan/pty_session_scan_types.rs"]
mod scan_types;
pub use scan_types::CapabilityScanActivity;
pub(super) use scan_types::{
    CapabilityCandidates, CapabilityScanAdvance, CapabilityScanResult, CapabilityScanStatus,
};
#[path = "../pty_session_bootstrap_exec.rs"]
mod bootstrap_exec;
#[path = "../pty_session_bootstrap_fence.rs"]
mod bootstrap_fence;
pub use bootstrap_exec::{BootstrapExecTransitionAuthority, ExplicitBootstrapExecFailureProof};
#[path = "pty_session_proof_capture.rs"]
mod capture;
#[path = "pty_session_proof_clone.rs"]
mod clone;
#[path = "../pty_session_master_alias_state.rs"]
mod master_alias_state;
use master_alias_state::PtyMasterAliasState;
#[path = "../reopen/pty_session_reopen_seal.rs"]
mod reopen_seal;
pub use reopen_seal::CommittedPtyReopenSealAuthority;
pub(in crate::process) use reopen_seal::PtyReopenSealAuthority;
#[path = "../pty_session_master_release.rs"]
mod master_release;
#[cfg(not(target_os = "macos"))]
#[path = "../pty_session_root_exit_noop.rs"]
mod root_exit_noop;
#[path = "../scan/pty_session_scan_state.rs"]
mod scan_state;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/pty_session_scope_tests.rs"]
mod scope_tests;
pub(super) fn requires_pty_object_lease(proof: Option<&PtySessionProof>) -> bool {
    proof.is_some_and(PtySessionProof::requires_pty_object_lease)
}
#[cfg(all(test, target_os = "macos", feature = "yolo-integration-tests"))]
include!("../../../../integration_tests/pty/session/reopen/pty_session_reopen_seal_test_hooks.rs");
impl StableInventory {
    pub(super) fn capability_candidates(
        &self,
        proof: Option<&PtySessionProof>,
        state: &PtyCapabilityScanState,
        exact_priorities: &HashSet<super::LiveProcessIdentity>,
        excluded: &HashSet<super::LiveProcessIdentity>,
        root: ProcessIdentity,
        root_lineage: u64,
        root_exited: bool,
        scan_escaped: bool,
    ) -> Result<CapabilityScanResult> {
        let Some(proof) = proof else {
            return Ok(CapabilityScanResult {
                candidates: CapabilityCandidates::default(),
                advance: CapabilityScanAdvance::reset(),
                scheduling_status: CapabilityScanStatus::Complete,
                coverage_status: CapabilityScanStatus::Complete,
            });
        };
        #[cfg(target_os = "macos")]
        {
            if let Some(result) = proof.bootstrap_capability_scan_result() {
                return Ok(result);
            }
            self.macos_capability_candidates(
                proof,
                state,
                exact_priorities,
                excluded,
                root.start_time,
                root_lineage,
                root_exited,
                scan_escaped,
            )
        }
        #[cfg(target_os = "linux")]
        {
            let _ = (root_lineage, scan_escaped);
            let context = (root, root_exited, exact_priorities, excluded);
            linux_scan::scan(self, proof, &state.linux, context)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            let _ = state;
            let candidates = CapabilityCandidates::default();
            let _ = (
                proof,
                exact_priorities,
                excluded,
                root,
                root_lineage,
                root_exited,
                scan_escaped,
            );
            Ok(CapabilityScanResult {
                candidates,
                advance: CapabilityScanAdvance::reset(),
                scheduling_status: CapabilityScanStatus::Complete,
                coverage_status: CapabilityScanStatus::Complete,
            })
        }
    }
}
#[cfg(target_os = "linux")]
#[path = "pty_session_proof_linux.rs"]
mod linux;
#[cfg(target_os = "linux")]
#[path = "../linux/pty_session_linux_custody.rs"]
mod linux_custody;
#[cfg(target_os = "linux")]
#[path = "../linux/pty_session_linux_holder.rs"]
mod linux_holder;
#[cfg(target_os = "linux")]
pub(in crate::process) use linux_custody::{LinuxPtyCustodyScanner, LinuxPtyCustodyState};
#[cfg(target_os = "linux")]
pub(in crate::process) use linux_holder::LinuxPtyHolderState;
#[cfg(target_os = "linux")]
#[path = "../linux/pty_session_linux_contract.rs"]
mod linux_contract;
#[cfg(target_os = "linux")]
#[path = "../linux/pty_session_linux_fds.rs"]
mod linux_fds;
#[cfg(target_os = "linux")]
#[path = "../linux/pty_session_linux_io.rs"]
mod linux_io;
#[cfg(target_os = "linux")]
#[path = "../scan/pty_session_scan_linux.rs"]
mod linux_scan;
#[cfg(target_os = "linux")]
#[path = "../scan/pty_session_scan_linux_state.rs"]
mod linux_scan_state;
include!("../pty_session_macos_wiring.rs");

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/pty_session_master_alias_state_tests.rs"]
mod master_alias_state_tests;
#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/pty_session_premature_release_fallback_tests.rs"]
mod premature_release_fallback_tests;
