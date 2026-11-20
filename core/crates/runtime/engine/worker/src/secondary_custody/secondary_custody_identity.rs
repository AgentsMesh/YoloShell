use super::ControllerGeneration;
use crate::pty_custody::{
    CustodyDurableLease, CustodyGuardianBinding, CustodyHolderIdentity, CustodyLeaseId,
    CustodyOffer, CustodyOfferId, CustodyProfile,
};
use crate::sha256_digest::Sha256Digest;

#[cfg(test)]
use crate::pty_custody::CustodyDescriptorSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ReadyWorkerCustodyIdentity {
    durable: CustodyDurableLease,
}

impl ReadyWorkerCustodyIdentity {
    pub(crate) fn new(
        offer: &CustodyOffer,
        holder: CustodyHolderIdentity,
        generation: u64,
        guardian: Option<CustodyGuardianBinding>,
    ) -> Self {
        Self {
            durable: CustodyDurableLease {
                lease_id: offer.lease_id,
                offer_id: offer.offer_id,
                holder,
                controller_generation: generation,
                descriptor_set: offer.descriptor_set,
                profile: offer.profile,
                recovery_identity_sha256: offer.recovery_identity_sha256,
                guardian,
            },
        }
    }

    pub(crate) fn generation(self) -> u64 {
        self.durable.controller_generation
    }

    pub(crate) fn controller_generation(self) -> ControllerGeneration {
        ControllerGeneration(self.generation())
    }

    pub(crate) fn lease_id(self) -> CustodyLeaseId {
        self.durable.lease_id
    }

    pub(crate) fn offer_id(self) -> CustodyOfferId {
        self.durable.offer_id
    }

    pub(crate) fn holder(self) -> CustodyHolderIdentity {
        self.durable.holder
    }

    pub(crate) fn recovery_identity_sha256(self) -> Sha256Digest {
        self.durable.recovery_identity_sha256
    }

    pub(crate) fn profile(self) -> CustodyProfile {
        self.durable.profile
    }

    pub(crate) fn durable_lease(self) -> CustodyDurableLease {
        self.durable
    }

    pub(crate) fn matches_durable(self, durable: CustodyDurableLease) -> bool {
        self.durable == durable
    }

    #[cfg(test)]
    pub(crate) fn fresh_for_test(generation: u64) -> Self {
        Self {
            durable: CustodyDurableLease {
                lease_id: CustodyLeaseId::new(),
                offer_id: CustodyOfferId::new(),
                holder: CustodyHolderIdentity::capture().unwrap(),
                controller_generation: generation,
                descriptor_set: CustodyDescriptorSet::LinuxContainment,
                profile: CustodyProfile::ActiveRunning,
                recovery_identity_sha256: Sha256Digest::repeated(TEST_DIGEST_BYTE),
                guardian: None,
            },
        }
    }

    pub(crate) fn from_durable(durable: CustodyDurableLease) -> Self {
        Self { durable }
    }

    #[cfg(test)]
    pub(crate) fn from_durable_for_test(durable: CustodyDurableLease) -> Self {
        Self::from_durable(durable)
    }

    #[cfg(test)]
    pub(crate) fn with_holder_for_test(mut self, holder: CustodyHolderIdentity) -> Self {
        self.durable.holder = holder;
        self
    }
}

#[cfg(test)]
const TEST_DIGEST_BYTE: u8 = 0x6b;
