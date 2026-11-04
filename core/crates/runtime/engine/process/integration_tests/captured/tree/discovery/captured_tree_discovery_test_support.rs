use super::inventory_test_support::TestProcess;
use super::{
    CapturedProcessTree, LiveProcessIdentity, MacosProcessCapabilityChildWriter,
    MacosProcessCapabilityIssuer, MacosProcessCapabilityManagerReader,
    MacosProcessCapabilityObservation, PROCESS_DISCOVERY_CAPABILITY_ENV,
};
use std::sync::Arc;
use std::time::{Duration, Instant};

const RELEASE_TIMEOUT: Duration = Duration::from_secs(10);
const RELEASE_POLL_INTERVAL: Duration = Duration::from_millis(1);
const RELEASE_TIMEOUT_DETAIL: &str = "discovery capability writer did not reach kernel EOF";

pub(super) struct DiscoveryTreeTestFixture {
    pub(super) root: TestProcess,
    pub(super) tree: CapturedProcessTree,
    authority: Arc<MacosProcessCapabilityManagerReader>,
    writer: Option<MacosProcessCapabilityChildWriter>,
    capability: String,
}

impl DiscoveryTreeTestFixture {
    pub(super) fn spawn(capability: &str) -> Self {
        let issuer = MacosProcessCapabilityIssuer::create().unwrap();
        let (authority, writer) = issuer.transfer_for_test().unwrap();
        let root = TestProcess::spawn_discovery_holder(
            PROCESS_DISCOVERY_CAPABILITY_ENV,
            capability,
            &writer,
        );
        let live = LiveProcessIdentity::capture(root.identity)
            .unwrap()
            .unwrap();
        authority.verify_writer_holder(&live).unwrap();
        let mut tree = CapturedProcessTree::capture_with_discovery_internal(
            root.identity,
            capability.to_owned(),
        )
        .unwrap();
        tree.install_process_capability(Arc::clone(&authority))
            .unwrap();
        Self {
            root,
            tree,
            authority,
            writer: Some(writer),
            capability: capability.to_owned(),
        }
    }

    pub(super) fn spawn_holder(&self) -> TestProcess {
        let holder = TestProcess::spawn_discovery_holder(
            PROCESS_DISCOVERY_CAPABILITY_ENV,
            &self.capability,
            self.writer.as_ref().unwrap(),
        );
        let live = LiveProcessIdentity::capture(holder.identity)
            .unwrap()
            .unwrap();
        self.authority.verify_writer_holder(&live).unwrap();
        holder
    }

    pub(super) fn release_local_writer(&mut self) {
        drop(self.writer.take());
    }

    pub(super) fn exit_root(&mut self) {
        self.release_local_writer();
        self.root.terminate();
        let proof = self.tree.root_exit_proof_for_test(self.root.identity);
        self.tree.mark_root_exited_unreaped(proof).unwrap();
    }

    pub(super) fn wait_for_kernel_eof(&self) {
        let expected = Some(MacosProcessCapabilityObservation::ReleasedAfterRootExit);
        let deadline = Instant::now() + RELEASE_TIMEOUT;
        while Instant::now() < deadline {
            if self.authority.liveness_observation() == expected {
                return;
            }
            std::thread::sleep(RELEASE_POLL_INTERVAL);
        }
        assert_eq!(
            self.authority.liveness_observation(),
            expected,
            "{RELEASE_TIMEOUT_DETAIL}"
        );
    }

    pub(super) fn into_tree(self) -> CapturedProcessTree {
        self.tree
    }
}
