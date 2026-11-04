use crate::pty_custody::record::guardian::candidate;
use crate::pty_custody::{
    CustodyDurableLease, CustodyDurableRecord, CustodyDurableState, CustodyGuardianBinding,
    CustodyOffer, CustodyProfile, CustodyRequest,
};

impl CustodyDurableRecord {
    pub fn supersede_prepared_guardian(
        &self,
        request: &CustodyRequest,
        offer: &CustodyOffer,
        guardian: CustodyGuardianBinding,
    ) -> Option<Self> {
        let CustodyDurableState::PreparedGuardian {
            predecessor,
            candidate: current,
        } = &self.state
        else {
            return None;
        };
        let successor = candidate(request, offer, guardian);
        let replacement = Self {
            schema_version: self.schema_version,
            runtime: self.runtime.clone(),
            state: CustodyDurableState::PreparedGuardian {
                predecessor: predecessor.clone(),
                candidate: successor,
            },
        };
        let exact = self.is_valid()
            && request.is_well_formed()
            && offer.matches_request(request)
            && guardian.matches(request, offer)
            && current.holder != successor.holder
            && current.lease_id != successor.lease_id
            && current.offer_id != successor.offer_id
            && successor.controller_generation > current.controller_generation
            && successor.descriptor_set == current.descriptor_set
            && successor.recovery_identity_sha256 == current.recovery_identity_sha256
            && profile_transition(current.profile, successor.profile)
            && replacement.is_valid();
        exact.then_some(replacement)
    }

    pub(crate) fn prepared_guardian_predecessor(&self) -> Option<Self> {
        let CustodyDurableState::PreparedGuardian { predecessor, .. } = &self.state else {
            return None;
        };
        Some(Self {
            schema_version: self.schema_version,
            runtime: self.runtime.clone(),
            state: predecessor.as_ref().clone(),
        })
    }

    pub fn prepared_guardian_predecessor_lease(&self) -> Option<CustodyDurableLease> {
        self.prepared_guardian_predecessor()?.armed_lease()
    }
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
#[path = "../../../unit_tests/guardian_supersede_matrix_tests.rs"]
mod matrix_tests;
