use crate::{
    CapturedProcessTree, DescendantObservation, LiveProcessIdentity, ProcessIdentity,
    PtyProcessScope, PtySessionProof,
};
use anyhow::Result;

const FIXTURE_SELF_TERMINATION_DETAIL: &str = "fixture cleanup cannot target itself";

impl LiveProcessIdentity {
    pub fn capture_for_test(identity: ProcessIdentity) -> Result<Option<Self>> {
        Self::capture(identity)
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub fn terminate_retained_fixture(&self) -> Result<bool> {
        anyhow::ensure!(
            self.pid() != std::process::id(),
            FIXTURE_SELF_TERMINATION_DETAIL
        );
        self.signal_for_test(libc::SIGKILL)
    }
}

impl PtyProcessScope {
    pub fn root_for_test(&self) -> ProcessIdentity {
        self.root()
    }
}

impl CapturedProcessTree {
    pub fn capture_pty_for_test(root: ProcessIdentity, proof: PtySessionProof) -> Result<Self> {
        Self::capture_pty(root, proof)
    }

    pub fn observe_live_identities_for_test(&mut self) -> Result<Vec<ProcessIdentity>> {
        self.observe_live_identities()
    }

    pub fn descendant_observation_for_test(&self) -> DescendantObservation {
        self.descendant_observation()
    }

    pub fn fail_next_refresh_for_test(&mut self, detail: &str) {
        self.fail_next_refresh(detail);
    }

    pub fn reconcile_retained_root_exit_for_test(&mut self) -> Result<()> {
        self.reconcile_retained_root_exit()
    }

    pub fn reconcile_retained_exact_empty_for_test(&mut self) -> Result<()> {
        self.reconcile_retained_exact_empty()
    }
}

impl PtySessionProof {
    pub fn capture_bound_for_test(
        master: &dyn portable_pty::MasterPty,
        capability: String,
    ) -> Result<Self> {
        Self::capture_bound(master, capability)
    }

    pub fn discovery_for_test(capability: String) -> Result<Self> {
        Self::discovery(capability)
    }

    #[cfg(target_os = "macos")]
    pub fn install_process_capability_for_test(
        &mut self,
        reader: std::sync::Arc<crate::MacosProcessCapabilityManagerReader>,
    ) -> Result<()> {
        self.install_process_capability(reader)
    }
}
