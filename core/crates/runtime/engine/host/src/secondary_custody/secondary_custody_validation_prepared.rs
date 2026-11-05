use crate::pty_custody::{
    CustodyDurableRecord, CustodyHolderObservation, CustodyOffer, CustodyProfile,
};

pub(super) fn accepts(offer: &CustodyOffer, durable: &CustodyDurableRecord) -> bool {
    let Some(predecessor) = durable.prepared_guardian_predecessor_lease() else {
        return false;
    };
    let Some(candidate) = durable.prepared_guardian_candidate() else {
        return false;
    };
    let Some(guardian) = candidate.guardian else {
        return false;
    };
    let guardian_known = matches!(
        guardian.process().observation(),
        CustodyHolderObservation::Current | CustodyHolderObservation::ProvenGone
    );
    candidate.holder.observation() == CustodyHolderObservation::ProvenGone
        && guardian_known
        && offer.descriptor_set == predecessor.descriptor_set
        && offer.descriptor_set == candidate.descriptor_set
        && offer.recovery_identity_sha256 == predecessor.recovery_identity_sha256
        && offer.recovery_identity_sha256 == candidate.recovery_identity_sha256
        && profile_transition(predecessor.profile, offer.profile)
        && profile_transition(candidate.profile, offer.profile)
}

fn profile_transition(predecessor: CustodyProfile, successor: CustodyProfile) -> bool {
    matches!(
        (predecessor, successor),
        (CustodyProfile::ActiveRunning, CustodyProfile::ActiveRunning)
            | (
                CustodyProfile::ActiveRunning,
                CustodyProfile::ActiveTerminalReadOnly
            )
            | (
                CustodyProfile::ActiveTerminalReadOnly,
                CustodyProfile::ActiveTerminalReadOnly
            )
    )
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/secondary_custody/secondary_custody_validation_prepared_tests.rs"]
mod tests;
