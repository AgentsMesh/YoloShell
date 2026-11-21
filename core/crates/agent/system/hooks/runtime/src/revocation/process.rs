use crate::sha256_digest::Sha256Digest;
use crate::{HookViolation, ProcessIdentity};

const OBSERVATION_DIGEST_DOMAIN: &str = "yolo.agent-hook.revocation-process-observation.v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessObservationRevocationProof {
    observation_sha256: Sha256Digest,
}

impl ProcessObservationRevocationProof {
    pub(crate) fn capture(
        observed: ProcessIdentity,
        violation: HookViolation,
    ) -> Result<Self, HookViolation> {
        let observation_sha256 = observation_digest(observed, violation)?;
        Ok(Self { observation_sha256 })
    }

    pub(crate) fn matches(self, observed: ProcessIdentity, violation: HookViolation) -> bool {
        observation_digest(observed, violation)
            .is_ok_and(|digest| digest == self.observation_sha256)
    }

    pub(crate) const fn digest(self) -> Sha256Digest {
        self.observation_sha256
    }

    pub(crate) const fn restored(observation_sha256: Sha256Digest) -> Self {
        Self { observation_sha256 }
    }
}

fn observation_digest(
    observed: ProcessIdentity,
    violation: HookViolation,
) -> Result<Sha256Digest, HookViolation> {
    Sha256Digest::capture(&(OBSERVATION_DIGEST_DOMAIN, observed, violation))
        .map_err(|_| HookViolation::InvalidEnvelope)
}
