use crate::pty_custody::tests::{descriptor_set, digest, request};
use crate::pty_custody::*;
#[path = "record_commit_constants.rs"]
mod constants;
pub use constants::*;

pub fn transaction(
    profile: CustodyProfile,
) -> (CustodyRequest, CustodyOffer, CustodyDurableRecord) {
    let request = request();
    let descriptors = if profile == CustodyProfile::ReleasedTerminal {
        CustodyDescriptorSet::ReleasedSuccessor
    } else {
        descriptor_set()
    };
    let offer = CustodyOffer::new_with_profile(&request, descriptors, profile, digest()).unwrap();
    let record = arm_fixture(
        &CustodyDurableRecord::pending(request.runtime.clone()),
        &request,
        &offer,
    )
    .unwrap();
    (request, offer, record)
}

pub fn assert_committed(
    record: &CustodyDurableRecord,
    request: &CustodyRequest,
    offer: &CustodyOffer,
) {
    assert!(record.is_valid());
    assert!(request.is_well_formed());
    assert!(offer.matches_request(request));
    assert!(record.matches_committed_offer(request, offer, record.armed_lease().unwrap().guardian));
}

pub fn assert_other_transaction_rejected(
    record: &CustodyDurableRecord,
    request: &CustodyRequest,
    offer: &CustodyOffer,
) {
    // Each candidate is internally valid: failure must come from its mismatch
    // against the durable receipt, not malformed input or a stale paired offer.
    assert!(record.is_valid());
    assert!(request.is_well_formed());
    assert!(offer.matches_request(request));
    assert!(
        !record.matches_committed_offer(request, offer, record.armed_lease().unwrap().guardian),
        "{TRANSACTION_CHANGED}"
    );
}
