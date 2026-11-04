use super::super::secondary_custody_authority::ReadyWorkerCustodyIdentity;
use super::super::{
    AuthenticatedController, ErrorBody, SystemMutationPermit, WorkerCustodyLease, WorkerState,
};
use super::SuccessorPromotionProfile;
use std::sync::Arc;

impl WorkerState {
    pub(in crate::worker) fn promote_ready_custody_for_test(
        &self,
        permit: &SystemMutationPermit,
        lease: &WorkerCustodyLease,
    ) -> std::result::Result<AuthenticatedController, ErrorBody> {
        let controller = self.promote_ready_custody_identity(
            permit,
            lease.identity(),
            SuccessorPromotionProfile::ActiveController,
            None,
        )?;
        self.pty_alias_custody
            .install_release_handle(lease.release_handle());
        Ok(controller)
    }

    pub(in crate::worker) async fn adopt_controller(
        self: &Arc<Self>,
        generation: u64,
    ) -> std::result::Result<AuthenticatedController, ErrorBody> {
        let permit = self.admit_system_mutation().await;
        self.promote_ready_custody_identity(
            &permit,
            ReadyWorkerCustodyIdentity::fresh_for_test(generation),
            SuccessorPromotionProfile::ActiveController,
            None,
        )
    }
}
