use super::WorkerState;
use super::secondary_custody_authority::ReadyWorkerCustodyIdentity;
use super::secondary_custody_authority::reconciliation::CommittedCustodyAdoption;
use crate::model::RuntimeRecord;
use crate::paths::identity::FileIdentity;
use crate::pty_custody::CustodyDurableRecord;
use std::sync::Arc;

pub(crate) struct CustodyAdoptionFence {
    state: Arc<WorkerState>,
    expected_authority: Option<ReadyWorkerCustodyIdentity>,
    armed: bool,
    committed: Option<CommittedCustodyAdoption>,
}

impl CustodyAdoptionFence {
    pub(crate) fn unarmed(state: Arc<WorkerState>) -> Self {
        let expected_authority = state.secondary_custody_authority.predecessor_identity();
        Self {
            state,
            expected_authority,
            armed: false,
            committed: None,
        }
    }

    pub(crate) fn armed(state: Arc<WorkerState>) -> Self {
        let expected_authority = state.secondary_custody_authority.predecessor_identity();
        Self {
            state,
            expected_authority,
            armed: true,
            committed: None,
        }
    }

    pub(crate) fn arm_committed(
        &mut self,
        runtime: &RuntimeRecord,
        durable: CustodyDurableRecord,
        identity: FileIdentity,
    ) {
        self.armed = true;
        self.committed = Some(CommittedCustodyAdoption::new(
            runtime.clone(),
            durable,
            identity,
        ));
    }

    pub(crate) fn advance_to_successor(&mut self, identity: ReadyWorkerCustodyIdentity) {
        self.expected_authority = Some(identity);
    }

    pub(crate) fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for CustodyAdoptionFence {
    fn drop(&mut self) {
        if self.armed {
            self.state
                .fence_failed_adoption_now(self.expected_authority);
            if let Some(committed) = self.committed.take() {
                self.state
                    .secondary_custody_authority
                    .retain_committed_adoption(self.expected_authority, committed);
            }
        }
    }
}
