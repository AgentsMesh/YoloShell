use super::{CustodyControl, CustodyControlAck, CustodyOffer, CustodyReleaseSettlement};
use crate::sha256_digest::Sha256Digest;
use anyhow::{Context, Result, ensure};

const CLOSURE_IDENTITY_DOMAIN: &str = "yolo/pty-secondary-custody-closure/v1";
const RELEASE_CONTROL_REQUIRED_CONTEXT: &str =
    "exact PTY custody Released acknowledgement requires a Release control";
const RELEASE_ACK_MISMATCH_DETAIL: &str =
    "Host PTY custody Released acknowledgement does not match its exact Release";

pub struct ExactCustodyReleasedAck {
    _private: (),
}

#[cfg(any(test, feature = "yolo-test-support"))]
impl ExactCustodyReleasedAck {
    pub fn for_test() -> Self {
        Self { _private: () }
    }
}

impl CustodyControl {
    pub fn release(offer: &CustodyOffer) -> Result<Self> {
        Self::release_with_settlement(offer, CustodyReleaseSettlement::exact())
    }

    pub fn release_with_settlement(
        offer: &CustodyOffer,
        settlement: CustodyReleaseSettlement,
    ) -> Result<Self> {
        settlement.ensure_valid(offer)?;
        Ok(Self::Release {
            lease_id: offer.lease_id,
            offer_id: offer.offer_id,
            closure_identity_sha256: closure_identity(offer)?,
            settlement,
        })
    }

    pub fn supersede(
        predecessor: &CustodyOffer,
        successor: &CustodyOffer,
        successor_generation: u64,
    ) -> Self {
        Self::Supersede {
            lease_id: predecessor.lease_id,
            offer_id: predecessor.offer_id,
            successor_lease_id: successor.lease_id,
            successor_offer_id: successor.offer_id,
            successor_generation,
        }
    }

    pub fn matches_release(&self, offer: &CustodyOffer) -> bool {
        let Self::Release {
            lease_id,
            offer_id,
            closure_identity_sha256,
            settlement,
        } = self
        else {
            return false;
        };
        *lease_id == offer.lease_id
            && *offer_id == offer.offer_id
            && closure_identity(offer).is_ok_and(|expected| expected == *closure_identity_sha256)
            && settlement.validates(offer)
    }

    pub fn release_settlement(&self) -> Option<&CustodyReleaseSettlement> {
        match self {
            Self::Release { settlement, .. } => Some(settlement),
            Self::Supersede { .. } => None,
        }
    }
}

impl CustodyControlAck {
    pub fn released(control: &CustodyControl) -> Option<Self> {
        let CustodyControl::Release {
            lease_id,
            offer_id,
            closure_identity_sha256,
            settlement,
        } = control
        else {
            return None;
        };
        Some(Self::Released {
            lease_id: *lease_id,
            offer_id: *offer_id,
            closure_identity_sha256: *closure_identity_sha256,
            settlement: settlement.clone(),
        })
    }

    pub fn superseded(control: &CustodyControl) -> Option<Self> {
        let CustodyControl::Supersede {
            lease_id,
            offer_id,
            successor_lease_id,
            successor_offer_id,
            successor_generation,
        } = control
        else {
            return None;
        };
        Some(Self::Superseded {
            lease_id: *lease_id,
            offer_id: *offer_id,
            successor_lease_id: *successor_lease_id,
            successor_offer_id: *successor_offer_id,
            successor_generation: *successor_generation,
        })
    }

    pub fn into_exact_released(self, expected: &CustodyControl) -> Result<ExactCustodyReleasedAck> {
        let expected = Self::released(expected).context(RELEASE_CONTROL_REQUIRED_CONTEXT)?;
        ensure!(self == expected, RELEASE_ACK_MISMATCH_DETAIL);
        Ok(ExactCustodyReleasedAck { _private: () })
    }
}

fn closure_identity(offer: &CustodyOffer) -> Result<Sha256Digest> {
    Sha256Digest::capture(&(CLOSURE_IDENTITY_DOMAIN, offer))
}
