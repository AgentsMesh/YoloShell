use super::{Fixture, records};
use crate::model::RuntimeRecord;
use crate::paths::identity::FileIdentity;
use crate::process::ProcessIdentityObservation;
use crate::pty_custody::{
    CustodyDurableRecord, CustodyGuardianPreparation, CustodyHolderIdentity, CustodyOffer,
    CustodyRequest,
};
use std::time::{Duration, Instant};

const CONVERGENCE_TIMEOUT: Duration = Duration::from_secs(8);
const CONVERGENCE_POLL_INTERVAL: Duration = Duration::from_millis(10);
const RECORD_TIMEOUT_DETAIL: &str = "guardian custody convergence timed out";
const RUNTIME_FENCE_FILE_NAME: &str = "runtime-fence.json";

#[derive(Clone, Copy)]
pub(in super::super) enum RuntimeFence {
    Retained,
    SameValueNewIdentity,
}

pub(in super::super) struct HostSuccessor {
    holder: CustodyHolderIdentity,
}

impl HostSuccessor {
    pub(in super::super) fn capture() -> Self {
        Self {
            holder: CustodyHolderIdentity::capture().unwrap(),
        }
    }

    pub(in super::super) fn holder(&self) -> CustodyHolderIdentity {
        self.holder
    }

    pub(in super::super) fn exchange(&self, fixture: &Fixture) -> (CustodyRequest, CustodyOffer) {
        records::successor_at(
            &fixture.runtime,
            self.holder,
            fixture.live.export.descriptor_set(),
            fixture.live.export.recovery_identity_sha256(),
            records::CANDIDATE_B_GENERATION,
        )
    }
}

impl Fixture {
    pub(in super::super) fn arm_a_then_host_exit<F>(
        &mut self,
        preparation: CustodyGuardianPreparation,
        runtime_fence: RuntimeFence,
        before_unlock: F,
    ) -> (CustodyDurableRecord, CustodyDurableRecord)
    where
        F: FnOnce(),
    {
        let (prepared, prepared_identity) = self.current();
        assert_eq!(prepared_identity, preparation.custody_file_identity());
        let armed = prepared
            .arm_exact_prepared_guardian(preparation.binding())
            .unwrap();
        let lifecycle = self
            .paths
            .try_acquire_session_lifecycle_lock(self.runtime.terminal_id)
            .unwrap()
            .unwrap();
        let guard =
            crate::paths::durable_mutation::SessionDurableMutationGuard::acquire_after_lifecycle_lock(
                &self.paths,
                self.namespace.lease(),
                self.runtime.terminal_id,
            )
            .unwrap();
        crate::pty_custody::replace_exact_record(
            &guard,
            &self.paths.pty_custody(self.runtime.terminal_id),
            &prepared,
            prepared_identity,
            &armed,
        )
        .unwrap();
        self.live.host.terminate();
        if matches!(runtime_fence, RuntimeFence::SameValueNewIdentity) {
            let runtime_path = self.paths.runtime_record(self.runtime.terminal_id);
            let replacement_path = runtime_path.with_file_name(RUNTIME_FENCE_FILE_NAME);
            self.namespace
                .write_json(&replacement_path, &self.runtime)
                .unwrap();
            std::fs::rename(replacement_path, &runtime_path).unwrap();
            self.namespace
                .file(&runtime_path)
                .unwrap()
                .sync_parent()
                .unwrap();
            let current = self.current_runtime_snapshot();
            assert_eq!(current.0, self.runtime);
            assert_ne!(current.1, self.runtime_identity);
        }
        before_unlock();
        drop(guard);
        drop(lifecycle);
        (prepared, armed)
    }

    pub(in super::super) fn arm(
        &self,
        preparation: CustodyGuardianPreparation,
        request: &CustodyRequest,
        offer: &CustodyOffer,
    ) -> CustodyDurableRecord {
        let (prepared, identity) = self.current();
        assert_eq!(identity, preparation.custody_file_identity());
        let armed = prepared
            .arm_prepared_guardian(request, offer, preparation.binding())
            .unwrap();
        crate::worker::secondary_custody_persistence::persist_transition(
            &self.namespace,
            &self.paths,
            &self.runtime,
            self.runtime_identity,
            &prepared,
            identity,
            &armed,
        )
        .unwrap();
        armed
    }

    pub(in super::super) fn publish_worker_runtime_promotion(
        &self,
        armed: &CustodyDurableRecord,
    ) -> RuntimeRecord {
        let (current, identity) = self.current();
        assert_eq!(current, *armed);
        crate::custody_promotion::publish_exact(
            self.namespace.lease(),
            &self.paths,
            &current,
            identity,
        )
        .unwrap()
    }

    pub(in super::super) fn current_runtime_snapshot(&self) -> (RuntimeRecord, FileIdentity) {
        self.namespace
            .read_json_with_identity(&self.paths.runtime_record(self.runtime.terminal_id))
            .unwrap()
    }

    pub(in super::super) fn assert_worker_and_child_current(&self) {
        assert_eq!(
            self.live.worker.identity().observe(),
            ProcessIdentityObservation::Matching
        );
        assert_eq!(
            self.live.child.identity().observe(),
            ProcessIdentityObservation::Matching
        );
    }

    pub(in super::super) fn wait_for_record(
        &self,
        expected: &CustodyDurableRecord,
    ) -> (CustodyDurableRecord, FileIdentity) {
        let deadline = Instant::now() + CONVERGENCE_TIMEOUT;
        loop {
            let observation = self
                .namespace
                .read_json_with_identity::<CustodyDurableRecord>(
                    &self.paths.pty_custody(self.runtime.terminal_id),
                );
            match observation {
                Ok(current) if current.0 == *expected => return current,
                observation => assert!(
                    Instant::now() < deadline,
                    "{RECORD_TIMEOUT_DETAIL}: {observation:?}"
                ),
            }
            std::thread::sleep(CONVERGENCE_POLL_INTERVAL);
        }
    }
}
