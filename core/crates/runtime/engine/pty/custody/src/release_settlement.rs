use super::{CustodyDescriptorSet, CustodyOffer};
use crate::sha256_digest::Sha256Digest;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

const FALLBACK_ATTESTATION_DOMAIN: &str =
    "yolo/pty-secondary-custody-confirmed-bootstrap-global-exclusion/v1";
const FALLBACK_PROFILE_DETAIL: &str =
    "confirmed-bootstrap global exclusion requires macOS process custody";
const FALLBACK_ATTESTATION_DETAIL: &str =
    "PTY custody fallback attestation does not match its exact offer";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CustodyReleaseSettlement {
    ExactProcessCapability,
    ConfirmedBootstrapGlobalExclusion { attestation_sha256: Sha256Digest },
}

impl CustodyReleaseSettlement {
    pub(crate) const fn exact() -> Self {
        Self::ExactProcessCapability
    }

    pub fn confirmed_bootstrap_global_exclusion(offer: &CustodyOffer) -> Result<Self> {
        ensure!(
            matches!(
                offer.descriptor_set,
                CustodyDescriptorSet::MacosProcessCapability { .. }
            ),
            FALLBACK_PROFILE_DETAIL
        );
        Ok(Self::ConfirmedBootstrapGlobalExclusion {
            attestation_sha256: fallback_attestation(offer)?,
        })
    }

    pub(crate) fn validates(&self, offer: &CustodyOffer) -> bool {
        match self {
            Self::ExactProcessCapability => true,
            Self::ConfirmedBootstrapGlobalExclusion { attestation_sha256 } => {
                matches!(
                    offer.descriptor_set,
                    CustodyDescriptorSet::MacosProcessCapability { .. }
                ) && fallback_attestation(offer)
                    .is_ok_and(|expected| expected == *attestation_sha256)
            }
        }
    }

    pub fn requires_confirmed_bootstrap_global_exclusion(&self) -> bool {
        matches!(self, Self::ConfirmedBootstrapGlobalExclusion { .. })
    }

    pub(crate) fn ensure_valid(&self, offer: &CustodyOffer) -> Result<()> {
        ensure!(self.validates(offer), FALLBACK_ATTESTATION_DETAIL);
        Ok(())
    }
}

fn fallback_attestation(offer: &CustodyOffer) -> Result<Sha256Digest> {
    Sha256Digest::capture(&(FALLBACK_ATTESTATION_DOMAIN, offer))
}
