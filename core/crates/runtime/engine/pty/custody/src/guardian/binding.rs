use crate::pty_custody::{
    CustodyDescriptorSet, CustodyHolderIdentity, CustodyLeaseId, CustodyOffer, CustodyOfferId,
    CustodyProfile, CustodyRequest, CustodyRuntimeIdentity,
};
use crate::sha256_digest::Sha256Digest;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub(in crate::pty_custody) const CUSTODY_GUARDIAN_BINDING_SCHEMA_VERSION: u16 = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustodyGuardianBinding {
    schema_version: u16,
    #[serde(with = "crate::serde_non_nil_uuid")]
    guardian_id: Uuid,
    process: CustodyHolderIdentity,
    host_holder: CustodyHolderIdentity,
    runtime_identity_sha256: Sha256Digest,
    controller_generation: u64,
    lease_id: CustodyLeaseId,
    offer_id: CustodyOfferId,
    descriptor_set: CustodyDescriptorSet,
    profile: CustodyProfile,
    recovery_identity_sha256: Sha256Digest,
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/guardian_binding_matrix_tests.rs"]
mod matrix_tests;

impl CustodyGuardianBinding {
    pub fn new(
        guardian_id: Uuid,
        process: CustodyHolderIdentity,
        request: &CustodyRequest,
        offer: &CustodyOffer,
    ) -> Option<Self> {
        let binding = Self {
            schema_version: CUSTODY_GUARDIAN_BINDING_SCHEMA_VERSION,
            guardian_id,
            process,
            host_holder: request.holder,
            runtime_identity_sha256: Sha256Digest::capture(&request.runtime).ok()?,
            controller_generation: request.controller_generation,
            lease_id: request.lease_id,
            offer_id: offer.offer_id,
            descriptor_set: offer.descriptor_set,
            profile: offer.profile,
            recovery_identity_sha256: offer.recovery_identity_sha256,
        };
        binding.matches(request, offer).then_some(binding)
    }

    pub fn matches(&self, request: &CustodyRequest, offer: &CustodyOffer) -> bool {
        self.is_well_formed()
            && offer.matches_request(request)
            && self.host_holder == request.holder
            && Some(self.runtime_identity_sha256) == Sha256Digest::capture(&request.runtime).ok()
            && self.controller_generation == request.controller_generation
            && self.lease_id == request.lease_id
            && self.offer_id == offer.offer_id
            && self.descriptor_set == offer.descriptor_set
            && self.profile == offer.profile
            && self.recovery_identity_sha256 == offer.recovery_identity_sha256
    }

    pub fn matches_runtime(&self, runtime: &CustodyRuntimeIdentity) -> bool {
        runtime.is_complete()
            && Some(self.runtime_identity_sha256) == Sha256Digest::capture(runtime).ok()
    }

    pub(in crate::pty_custody) fn rewrite_runtime_identity(
        self,
        source: &CustodyRuntimeIdentity,
        target: &CustodyRuntimeIdentity,
    ) -> Option<Self> {
        if !self.matches_runtime(source) || !target.is_complete() {
            return None;
        }
        let mut replacement = self;
        replacement.runtime_identity_sha256 = Sha256Digest::capture(target).ok()?;
        (replacement.is_well_formed() && replacement.matches_runtime(target)).then_some(replacement)
    }

    pub fn matches_durable_lease(
        &self,
        runtime: &CustodyRuntimeIdentity,
        lease: &crate::pty_custody::CustodyDurableLease,
    ) -> bool {
        self.matches_runtime(runtime) && self.matches_lease_identity(lease)
    }

    pub fn matches_lease_identity(&self, lease: &crate::pty_custody::CustodyDurableLease) -> bool {
        lease.guardian == Some(*self)
            && self.host_holder == lease.holder
            && self.controller_generation == lease.controller_generation
            && self.lease_id == lease.lease_id
            && self.offer_id == lease.offer_id
            && self.descriptor_set == lease.descriptor_set
            && self.profile == lease.profile
            && self.recovery_identity_sha256 == lease.recovery_identity_sha256
    }

    pub fn process(self) -> CustodyHolderIdentity {
        self.process
    }

    pub fn guardian_id(self) -> Uuid {
        self.guardian_id
    }

    pub fn host_holder(self) -> CustodyHolderIdentity {
        self.host_holder
    }

    pub fn controller_generation(self) -> u64 {
        self.controller_generation
    }

    pub(crate) fn is_well_formed(&self) -> bool {
        self.schema_version == CUSTODY_GUARDIAN_BINDING_SCHEMA_VERSION
            && self.guardian_id != Uuid::nil()
            && self.process.pid != u32::default()
            && self.process.start_time != u64::default()
            && self.host_holder.pid != u32::default()
            && self.host_holder.start_time != u64::default()
            && self.controller_generation != u64::default()
    }
}
