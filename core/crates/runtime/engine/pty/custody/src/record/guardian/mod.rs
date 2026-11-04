pub(in crate::pty_custody) mod supersede;

use crate::pty_custody::{
    CustodyDurableLease, CustodyDurableRecord, CustodyDurableState, CustodyGuardianBinding,
    CustodyOffer, CustodyRequest,
};

#[cfg(test)]
use crate::pty_custody::CustodyDescriptorSet;
#[cfg(test)]
use uuid::Uuid;

impl CustodyDurableRecord {
    pub fn prepare_guardian(
        &self,
        request: &CustodyRequest,
        offer: &CustodyOffer,
        guardian: CustodyGuardianBinding,
    ) -> Option<Self> {
        let candidate = candidate(request, offer, guardian);
        let predecessor_allows = match self.state {
            CustodyDurableState::Pending {} => true,
            CustodyDurableState::Armed { .. } => self
                .lease()
                .is_some_and(|lease| request.controller_generation > lease.controller_generation),
            CustodyDurableState::PreparedGuardian { .. } | CustodyDurableState::Retired { .. } => {
                false
            }
        };
        let valid = self.is_valid()
            && predecessor_allows
            && self.runtime == request.runtime
            && request.is_well_formed()
            && offer.matches_request(request)
            && guardian.matches(request, offer);
        let replacement = Self {
            schema_version: self.schema_version,
            runtime: self.runtime.clone(),
            state: CustodyDurableState::PreparedGuardian {
                predecessor: Box::new(self.state.clone()),
                candidate,
            },
        };
        (valid && replacement.is_valid()).then_some(replacement)
    }

    pub fn arm_prepared_guardian(
        &self,
        request: &CustodyRequest,
        offer: &CustodyOffer,
        guardian: CustodyGuardianBinding,
    ) -> Option<Self> {
        let expected = candidate(request, offer, guardian);
        let CustodyDurableState::PreparedGuardian { candidate, .. } = self.state else {
            return None;
        };
        (self.is_valid() && candidate == expected).then(|| self.with_armed(expected))
    }

    pub fn arm_exact_prepared_guardian(&self, guardian: CustodyGuardianBinding) -> Option<Self> {
        let candidate = self.prepared_guardian_candidate()?;
        (self.is_valid() && candidate.guardian == Some(guardian))
            .then(|| self.with_armed(candidate))
    }

    pub fn abandon_prepared_guardian(&self, guardian: CustodyGuardianBinding) -> Option<Self> {
        let CustodyDurableState::PreparedGuardian {
            ref predecessor,
            candidate,
        } = self.state
        else {
            return None;
        };
        (self.is_valid() && candidate.guardian == Some(guardian)).then(|| Self {
            schema_version: self.schema_version,
            runtime: self.runtime.clone(),
            state: predecessor.as_ref().clone(),
        })
    }

    pub fn prepared_guardian_candidate(&self) -> Option<CustodyDurableLease> {
        match self.state {
            CustodyDurableState::PreparedGuardian { candidate, .. } => Some(candidate),
            _ => None,
        }
    }

    pub(in crate::pty_custody) fn with_armed(&self, lease: CustodyDurableLease) -> Self {
        Self {
            schema_version: self.schema_version,
            runtime: self.runtime.clone(),
            state: CustodyDurableState::Armed {
                lease_id: lease.lease_id,
                offer_id: lease.offer_id,
                holder: lease.holder,
                controller_generation: lease.controller_generation,
                descriptor_set: lease.descriptor_set,
                profile: lease.profile,
                recovery_identity_sha256: lease.recovery_identity_sha256,
                guardian: lease.guardian,
            },
        }
    }
}

pub(in crate::pty_custody) fn candidate(
    request: &CustodyRequest,
    offer: &CustodyOffer,
    guardian: CustodyGuardianBinding,
) -> CustodyDurableLease {
    CustodyDurableLease {
        lease_id: request.lease_id,
        offer_id: offer.offer_id,
        holder: request.holder,
        controller_generation: request.controller_generation,
        descriptor_set: offer.descriptor_set,
        profile: offer.profile,
        recovery_identity_sha256: offer.recovery_identity_sha256,
        guardian: Some(guardian),
    }
}
