use crate::pty_custody::{
    CustodyDescriptorSet, CustodyGuardianPreparation, CustodyHostAck, CustodyOffer, CustodyOfferId,
    CustodyProfile, CustodyRequest, CustodyWorkerActivated, CustodyWorkerCommit,
};
use crate::sha256_digest::Sha256Digest;
use anyhow::{Result, ensure};

const ACTIVATION_CONSTRUCTION_DETAIL: &str = "cannot construct an invalid PTY custody activation";
const ACTIVATION_MISMATCH_DETAIL: &str =
    "Worker PTY custody activation does not match its exact Ready offer";
const OFFER_PROFILE_DETAIL: &str = "PTY custody profile does not match its descriptor set";

impl CustodyOffer {
    pub(crate) fn new(
        request: &CustodyRequest,
        descriptor_set: CustodyDescriptorSet,
        recovery_identity_sha256: Sha256Digest,
    ) -> Self {
        Self {
            protocol_version: request.protocol_version,
            lease_id: request.lease_id,
            offer_id: CustodyOfferId::new(),
            runtime: request.runtime.clone(),
            descriptor_set,
            profile: CustodyProfile::for_descriptor_set(descriptor_set),
            recovery_identity_sha256,
        }
    }

    pub fn new_with_profile(
        request: &CustodyRequest,
        descriptor_set: CustodyDescriptorSet,
        profile: CustodyProfile,
        recovery_identity_sha256: Sha256Digest,
    ) -> Result<Self> {
        ensure!(
            profile.accepts_descriptor_set(descriptor_set),
            OFFER_PROFILE_DETAIL
        );
        let mut offer = Self::new(request, descriptor_set, recovery_identity_sha256);
        offer.profile = profile;
        Ok(offer)
    }
}

impl CustodyHostAck {
    pub fn prepared(offer: &CustodyOffer) -> Self {
        Self::Prepared {
            lease_id: offer.lease_id,
            offer_id: offer.offer_id,
            observed_identity_sha256: offer.recovery_identity_sha256,
            guardian: None,
        }
    }

    pub fn prepared_guarded(offer: &CustodyOffer, guardian: CustodyGuardianPreparation) -> Self {
        Self::Prepared {
            lease_id: offer.lease_id,
            offer_id: offer.offer_id,
            observed_identity_sha256: offer.recovery_identity_sha256,
            guardian: Some(guardian),
        }
    }

    pub fn ready(offer: &CustodyOffer, controller_generation: u64) -> Self {
        Self::Ready {
            lease_id: offer.lease_id,
            offer_id: offer.offer_id,
            controller_generation,
        }
    }

    pub fn matches_prepared(&self, offer: &CustodyOffer) -> bool {
        matches!(
            self,
            Self::Prepared {
                lease_id,
                offer_id,
                observed_identity_sha256,
                ..
            } if *lease_id == offer.lease_id
                && *offer_id == offer.offer_id
                && *observed_identity_sha256 == offer.recovery_identity_sha256
        )
    }

    pub fn prepared_guardian(&self) -> Option<CustodyGuardianPreparation> {
        match self {
            Self::Prepared { guardian, .. } => *guardian,
            Self::Ready { .. } => None,
        }
    }

    pub fn matches_ready(&self, offer: &CustodyOffer, generation: u64) -> bool {
        matches!(
            self,
            Self::Ready {
                lease_id,
                offer_id,
                controller_generation,
            } if *lease_id == offer.lease_id
                && *offer_id == offer.offer_id
                && *controller_generation == generation
        )
    }
}

impl CustodyWorkerCommit {
    pub fn new(offer: &CustodyOffer) -> Self {
        Self {
            protocol_version: offer.protocol_version,
            lease_id: offer.lease_id,
            offer_id: offer.offer_id,
            observed_identity_sha256: offer.recovery_identity_sha256,
        }
    }

    pub fn matches_offer(&self, offer: &CustodyOffer) -> bool {
        self.protocol_version == offer.protocol_version
            && self.lease_id == offer.lease_id
            && self.offer_id == offer.offer_id
            && self.observed_identity_sha256 == offer.recovery_identity_sha256
    }
}

pub struct ExactCustodyWorkerActivation {
    _private: (),
}

impl CustodyWorkerActivated {
    pub fn new(offer: &CustodyOffer, controller_generation: u64) -> Result<Self> {
        let activated = Self {
            protocol_version: offer.protocol_version,
            lease_id: offer.lease_id,
            offer_id: offer.offer_id,
            controller_generation,
        };
        ensure!(
            activated.matches(offer, controller_generation),
            ACTIVATION_CONSTRUCTION_DETAIL
        );
        Ok(activated)
    }

    pub fn into_exact(
        self,
        offer: &CustodyOffer,
        controller_generation: u64,
    ) -> Result<ExactCustodyWorkerActivation> {
        ensure!(
            self.matches(offer, controller_generation),
            ACTIVATION_MISMATCH_DETAIL
        );
        Ok(ExactCustodyWorkerActivation { _private: () })
    }

    fn matches(&self, offer: &CustodyOffer, generation: u64) -> bool {
        self.protocol_version == offer.protocol_version
            && self.lease_id == offer.lease_id
            && self.offer_id == offer.offer_id
            && self.controller_generation == generation
            && generation != u64::default()
    }
}
