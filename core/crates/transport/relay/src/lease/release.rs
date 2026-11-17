use super::super::constants::{
    LEASE_CLEANUP_POLL_ATTEMPTS, LEASE_CLEANUP_POLL_INTERVAL, LEASE_CLEANUP_TIMEOUT,
};
use super::LeaseController;
use crate::model::{WriterLease, WriterLeaseGrant};

impl LeaseController {
    pub(crate) async fn release(&mut self) {
        if self.is_terminal() {
            self.lease = None;
            self.ownership_uncertain = false;
            return;
        }
        let cleanup = self.release_inner();
        let _ = tokio::time::timeout(LEASE_CLEANUP_TIMEOUT, cleanup).await;
        self.lease = None;
        self.ownership_uncertain = false;
    }

    async fn release_inner(&mut self) {
        if self.is_terminal() {
            return;
        }
        if let Some(grant) = self.lease.take() {
            self.release_known(&grant).await;
            return;
        }
        if !self.ownership_uncertain {
            return;
        }
        for _ in 0..LEASE_CLEANUP_POLL_ATTEMPTS {
            if self.is_terminal() {
                return;
            }
            match self.runtime.client().get_writer_lease(self.target).await {
                Ok(Some(lease)) if self.is_ours(&lease) => {
                    if let Some(grant) = self.recover_grant(&lease).await {
                        self.release_known(&grant).await;
                    }
                    return;
                }
                Ok(Some(_)) | Err(_) => return,
                Ok(None) => tokio::time::sleep(LEASE_CLEANUP_POLL_INTERVAL).await,
            }
        }
    }

    async fn recover_grant(&self, lease: &WriterLease) -> Option<WriterLeaseGrant> {
        let grant = self.request_acquire().await.ok()?;
        (same_lease_incarnation(&grant.lease, lease)
            && grant.writer_capability == self.writer_capability)
            .then_some(grant)
    }

    async fn release_known(&self, grant: &WriterLeaseGrant) {
        let _ = self
            .runtime
            .client()
            .release_writer(grant, self.target)
            .await;
    }
}

pub(super) fn same_lease_incarnation(left: &WriterLease, right: &WriterLease) -> bool {
    left.holder_id == right.holder_id
        && left.controller_instance_id == right.controller_instance_id
        && left.generation == right.generation
}
