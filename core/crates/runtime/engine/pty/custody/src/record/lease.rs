use crate::pty_custody::{
    CustodyDurableLease, CustodyDurableRecord, CustodyDurableState, CustodyOffer,
};

impl CustodyDurableRecord {
    pub fn matches_candidate(&self, offer: &CustodyOffer, expected: CustodyDurableLease) -> bool {
        self.is_valid()
            && self.runtime == offer.runtime
            && expected.lease_id == offer.lease_id
            && expected.offer_id == offer.offer_id
            && expected.descriptor_set == offer.descriptor_set
            && expected.profile == offer.profile
            && expected.recovery_identity_sha256 == offer.recovery_identity_sha256
            && self.armed_lease() == Some(expected)
    }

    pub(crate) fn retire(&self, expected: CustodyDurableLease) -> Option<Self> {
        self.retire_with_optional_evidence(expected, None)
    }

    pub fn retire_with_evidence(
        &self,
        expected: CustodyDurableLease,
        retirement_evidence: crate::pty_custody::CustodyRetirementEvidence,
    ) -> Option<Self> {
        self.retire_with_optional_evidence(expected, Some(retirement_evidence))
    }

    fn retire_with_optional_evidence(
        &self,
        expected: CustodyDurableLease,
        retirement_evidence: Option<crate::pty_custody::CustodyRetirementEvidence>,
    ) -> Option<Self> {
        let replacement = Self {
            schema_version: self.schema_version,
            runtime: self.runtime.clone(),
            state: CustodyDurableState::Retired {
                lease_id: expected.lease_id,
                offer_id: expected.offer_id,
                holder: expected.holder,
                controller_generation: expected.controller_generation,
                descriptor_set: expected.descriptor_set,
                profile: expected.profile,
                recovery_identity_sha256: expected.recovery_identity_sha256,
                guardian: expected.guardian,
                retirement_evidence,
            },
        };
        (self.is_valid() && self.armed_lease() == Some(expected) && replacement.is_valid())
            .then_some(replacement)
    }

    pub fn armed_lease(&self) -> Option<CustodyDurableLease> {
        match self.state {
            CustodyDurableState::Armed { .. } => self.lease(),
            CustodyDurableState::Pending {}
            | CustodyDurableState::PreparedGuardian { .. }
            | CustodyDurableState::Retired { .. } => None,
        }
    }

    pub fn retired_lease(&self) -> Option<CustodyDurableLease> {
        match self.state {
            CustodyDurableState::Retired { .. } => self.lease(),
            CustodyDurableState::Pending {}
            | CustodyDurableState::PreparedGuardian { .. }
            | CustodyDurableState::Armed { .. } => None,
        }
    }

    pub fn retirement_evidence(&self) -> Option<crate::pty_custody::CustodyRetirementEvidence> {
        match self.state {
            CustodyDurableState::Retired {
                retirement_evidence,
                ..
            } => retirement_evidence,
            CustodyDurableState::Pending {}
            | CustodyDurableState::PreparedGuardian { .. }
            | CustodyDurableState::Armed { .. } => None,
        }
    }

    pub fn retire_prepared_guardian_with_evidence(
        &self,
        expected: CustodyDurableLease,
        evidence: crate::pty_custody::CustodyRetirementEvidence,
    ) -> Option<Self> {
        (self.is_valid()
            && self.prepared_guardian_candidate() == Some(expected)
            && evidence.is_valid())
        .then(|| self.with_armed(expected))
        .and_then(|armed| armed.retire_with_evidence(expected, evidence))
    }

    pub fn predecessor_lease(&self) -> Option<CustodyDurableLease> {
        self.lease()
    }

    pub(in crate::pty_custody) fn lease(&self) -> Option<CustodyDurableLease> {
        let (lease_id, offer_id, holder, generation, descriptors, profile, digest, guardian) =
            match self.state {
                CustodyDurableState::Pending {} | CustodyDurableState::PreparedGuardian { .. } => {
                    return None;
                }
                CustodyDurableState::Armed {
                    lease_id,
                    offer_id,
                    holder,
                    controller_generation,
                    descriptor_set,
                    profile,
                    recovery_identity_sha256,
                    guardian,
                    ..
                }
                | CustodyDurableState::Retired {
                    lease_id,
                    offer_id,
                    holder,
                    controller_generation,
                    descriptor_set,
                    profile,
                    recovery_identity_sha256,
                    guardian,
                    ..
                } => (
                    lease_id,
                    offer_id,
                    holder,
                    controller_generation,
                    descriptor_set,
                    profile,
                    recovery_identity_sha256,
                    guardian,
                ),
            };
        Some(CustodyDurableLease {
            lease_id,
            offer_id,
            holder,
            controller_generation: generation,
            descriptor_set: descriptors,
            profile,
            recovery_identity_sha256: digest,
            guardian,
        })
    }
}
