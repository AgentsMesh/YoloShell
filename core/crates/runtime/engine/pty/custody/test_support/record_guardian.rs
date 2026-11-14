use super::*;

pub fn arm_fixture(
    pending: &CustodyDurableRecord,
    request: &CustodyRequest,
    offer: &CustodyOffer,
) -> Option<CustodyDurableRecord> {
    let CustodyDescriptorSet::MacosProcessCapability { .. } = offer.descriptor_set else {
        return pending.arm(request, offer);
    };
    let guardian = CustodyGuardianBinding::new(Uuid::new_v4(), request.holder, request, offer)?;
    let prepared = pending.prepare_guardian(request, offer, guardian)?;
    prepared.arm_prepared_guardian(request, offer, guardian)
}

pub fn rearm_fixture(
    predecessor: &CustodyDurableRecord,
    request: &CustodyRequest,
    offer: &CustodyOffer,
) -> Option<CustodyDurableRecord> {
    let CustodyDescriptorSet::MacosProcessCapability { .. } = offer.descriptor_set else {
        return predecessor.rearm(request, offer);
    };
    let guardian = CustodyGuardianBinding::new(Uuid::new_v4(), request.holder, request, offer)?;
    let prepared = predecessor.prepare_guardian(request, offer, guardian)?;
    prepared.arm_prepared_guardian(request, offer, guardian)
}
