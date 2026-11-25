#[cfg(test)]
pub(in crate::pty_custody) mod constants;
pub(in crate::pty_custody) mod request;
pub(in crate::pty_custody) mod state;

use crate::pty_custody::{
    CustodyDescriptorSet, CustodyGuardianPreparation, CustodyHolderIdentity, CustodyLeaseId,
    CustodyOfferId, CustodyProfile, CustodyReleaseSettlement, CustodyRuntimeIdentity,
    PROTOCOL_VERSION,
};
use crate::sha256_digest::Sha256Digest;
use serde::{Deserialize, Serialize};

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustodyRequest {
    pub protocol_version: u16,
    pub lease_id: CustodyLeaseId,
    pub runtime: CustodyRuntimeIdentity,
    pub(crate) holder: CustodyHolderIdentity,
    pub(crate) auth_token: String,
    pub controller_generation: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustodyOffer {
    pub protocol_version: u16,
    pub lease_id: CustodyLeaseId,
    pub offer_id: CustodyOfferId,
    pub runtime: CustodyRuntimeIdentity,
    pub descriptor_set: CustodyDescriptorSet,
    pub profile: CustodyProfile,
    pub recovery_identity_sha256: Sha256Digest,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case", deny_unknown_fields)]
pub enum CustodyHostAck {
    Prepared {
        lease_id: CustodyLeaseId,
        offer_id: CustodyOfferId,
        observed_identity_sha256: Sha256Digest,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        guardian: Option<CustodyGuardianPreparation>,
    },
    Ready {
        lease_id: CustodyLeaseId,
        offer_id: CustodyOfferId,
        controller_generation: u64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustodyWorkerCommit {
    pub protocol_version: u16,
    pub lease_id: CustodyLeaseId,
    pub offer_id: CustodyOfferId,
    pub(crate) observed_identity_sha256: Sha256Digest,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustodyWorkerActivated {
    pub protocol_version: u16,
    pub lease_id: CustodyLeaseId,
    pub offer_id: CustodyOfferId,
    pub controller_generation: u64,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case", deny_unknown_fields)]
pub enum CustodyWorkerReadyDisposition {
    Activated { activation: CustodyWorkerActivated },
    Release { control: CustodyControl },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CustodyControl {
    Release {
        lease_id: CustodyLeaseId,
        offer_id: CustodyOfferId,
        closure_identity_sha256: Sha256Digest,
        settlement: CustodyReleaseSettlement,
    },
    Supersede {
        lease_id: CustodyLeaseId,
        offer_id: CustodyOfferId,
        successor_lease_id: CustodyLeaseId,
        successor_offer_id: CustodyOfferId,
        successor_generation: u64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CustodyControlAck {
    Released {
        lease_id: CustodyLeaseId,
        offer_id: CustodyOfferId,
        closure_identity_sha256: Sha256Digest,
        settlement: CustodyReleaseSettlement,
    },
    Superseded {
        lease_id: CustodyLeaseId,
        offer_id: CustodyOfferId,
        successor_lease_id: CustodyLeaseId,
        successor_offer_id: CustodyOfferId,
        successor_generation: u64,
    },
}

impl CustodyWorkerReadyDisposition {
    pub fn activated(activation: CustodyWorkerActivated) -> Self {
        Self::Activated { activation }
    }

    pub fn release(control: CustodyControl) -> Self {
        Self::Release { control }
    }
}

impl CustodyRequest {
    pub fn new(
        lease_id: CustodyLeaseId,
        runtime: CustodyRuntimeIdentity,
        holder: CustodyHolderIdentity,
        auth_token: String,
        controller_generation: u64,
    ) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            lease_id,
            runtime,
            holder,
            auth_token,
            controller_generation,
        }
    }

    pub fn is_well_formed(&self) -> bool {
        supported_protocol(self.protocol_version)
            && self.runtime.is_complete()
            && self.holder.pid != u32::default()
            && self.holder.start_time != u64::default()
            && !self.auth_token.is_empty()
            && self.controller_generation != u64::default()
    }

    pub fn uses_current_protocol(&self) -> bool {
        self.protocol_version == PROTOCOL_VERSION
    }
}

impl CustodyOffer {
    pub fn matches_request(&self, request: &CustodyRequest) -> bool {
        self.protocol_version == request.protocol_version
            && supported_protocol(self.protocol_version)
            && self.lease_id == request.lease_id
            && self.runtime == request.runtime
            && self.runtime.is_complete()
            && self.descriptor_set.is_well_formed()
            && self.profile.accepts_descriptor_set(self.descriptor_set)
    }
}

const fn supported_protocol(version: u16) -> bool {
    version == PROTOCOL_VERSION
}
