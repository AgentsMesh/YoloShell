use crate::pty_custody::{
    CustodyDurableRecord, CustodyDurableState, CustodyGuardianBinding, CustodyOffer, CustodyRequest,
};

impl CustodyDurableRecord {
    pub fn matches_committed_offer(
        &self,
        request: &CustodyRequest,
        offer: &CustodyOffer,
        guardian: Option<CustodyGuardianBinding>,
    ) -> bool {
        self.is_valid()
            && request.is_well_formed()
            && self.runtime == request.runtime
            && offer.matches_request(request)
            && matches!(
                &self.state,
                CustodyDurableState::Armed {
                    lease_id,
                    offer_id,
                    holder,
                    controller_generation,
                    descriptor_set,
                    profile,
                    recovery_identity_sha256,
                    guardian: durable_guardian,
                } if *lease_id == request.lease_id
                    && *offer_id == offer.offer_id
                    && *holder == request.holder
                    && *controller_generation == request.controller_generation
                    && *descriptor_set == offer.descriptor_set
                    && *profile == offer.profile
                    && *recovery_identity_sha256 == offer.recovery_identity_sha256
                    && *durable_guardian == guardian
            )
    }
}
