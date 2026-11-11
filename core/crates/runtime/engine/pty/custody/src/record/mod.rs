pub(in crate::pty_custody) mod commit;
pub(in crate::pty_custody) mod guardian;
pub(in crate::pty_custody) mod lease;
pub(in crate::pty_custody) mod transition;
pub(in crate::pty_custody) mod validation;
pub(in crate::pty_custody) mod wire;

use crate::pty_custody::{
    CustodyDescriptorSet, CustodyGuardianBinding, CustodyHolderIdentity, CustodyLeaseId,
    CustodyOffer, CustodyOfferId, CustodyProfile, CustodyRequest, CustodyRetirementEvidence,
    CustodyRuntimeIdentity,
};
use crate::sha256_digest::Sha256Digest;
use serde::{Deserialize, Serialize};

pub(in crate::pty_custody) const CUSTODY_RECORD_SCHEMA_VERSION: u16 = 5;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CustodyDurableRecord {
    pub schema_version: u16,
    pub runtime: CustodyRuntimeIdentity,
    pub state: CustodyDurableState,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CustodyDurableState {
    Pending {},
    PreparedGuardian {
        predecessor: Box<CustodyDurableState>,
        candidate: CustodyDurableLease,
    },
    Armed {
        lease_id: CustodyLeaseId,
        offer_id: CustodyOfferId,
        holder: CustodyHolderIdentity,
        controller_generation: u64,
        descriptor_set: CustodyDescriptorSet,
        profile: CustodyProfile,
        recovery_identity_sha256: Sha256Digest,
        guardian: Option<CustodyGuardianBinding>,
    },
    Retired {
        lease_id: CustodyLeaseId,
        offer_id: CustodyOfferId,
        holder: CustodyHolderIdentity,
        controller_generation: u64,
        descriptor_set: CustodyDescriptorSet,
        profile: CustodyProfile,
        recovery_identity_sha256: Sha256Digest,
        guardian: Option<CustodyGuardianBinding>,
        retirement_evidence: Option<CustodyRetirementEvidence>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustodyDurableLease {
    pub lease_id: CustodyLeaseId,
    pub offer_id: CustodyOfferId,
    pub holder: CustodyHolderIdentity,
    pub controller_generation: u64,
    pub descriptor_set: CustodyDescriptorSet,
    pub profile: CustodyProfile,
    pub recovery_identity_sha256: Sha256Digest,
    pub guardian: Option<CustodyGuardianBinding>,
}

impl CustodyDurableRecord {
    pub fn pending(runtime: CustodyRuntimeIdentity) -> Self {
        Self {
            schema_version: CUSTODY_RECORD_SCHEMA_VERSION,
            runtime,
            state: CustodyDurableState::Pending {},
        }
    }

    pub fn arm(&self, request: &CustodyRequest, offer: &CustodyOffer) -> Option<Self> {
        let valid = self.is_valid()
            && matches!(self.state, CustodyDurableState::Pending {})
            && self.runtime == request.runtime
            && offer.matches_request(request)
            && request.is_well_formed();
        let replacement = Self {
            schema_version: self.schema_version,
            runtime: self.runtime.clone(),
            state: CustodyDurableState::Armed {
                lease_id: request.lease_id,
                offer_id: offer.offer_id,
                holder: request.holder,
                controller_generation: request.controller_generation,
                descriptor_set: offer.descriptor_set,
                profile: offer.profile,
                recovery_identity_sha256: offer.recovery_identity_sha256,
                guardian: None,
            },
        };
        (valid && replacement.is_valid()).then_some(replacement)
    }

    pub fn rearm(&self, request: &CustodyRequest, offer: &CustodyOffer) -> Option<Self> {
        let Some(predecessor) = self.lease() else {
            return None;
        };
        let valid = self.is_valid()
            && self.runtime == request.runtime
            && offer.matches_request(request)
            && request.is_well_formed()
            && request.controller_generation > predecessor.controller_generation;
        let replacement = Self {
            schema_version: self.schema_version,
            runtime: self.runtime.clone(),
            state: CustodyDurableState::Armed {
                lease_id: request.lease_id,
                offer_id: offer.offer_id,
                holder: request.holder,
                controller_generation: request.controller_generation,
                descriptor_set: offer.descriptor_set,
                profile: offer.profile,
                recovery_identity_sha256: offer.recovery_identity_sha256,
                guardian: None,
            },
        };
        (valid && replacement.is_valid()).then_some(replacement)
    }
}
