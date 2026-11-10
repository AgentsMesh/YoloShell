use crate::reducer::Reducer;
use crate::sha256_digest::Sha256Digest;
use crate::snapshot::SnapshotViolation;
use crate::snapshot::revision::AuthorityRevision;
use crate::state::NormalizedLifecycle;

const PREDECESSOR_DIGEST_DOMAIN: &str = "yolo.agent-hook.revocation-predecessor.v1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreRevocationProof {
    state: Box<NormalizedLifecycle>,
    revision: AuthorityRevision,
    snapshot_sha256: Sha256Digest,
}

impl PreRevocationProof {
    pub(crate) fn capture(reducer: &Reducer) -> Result<Self, crate::HookViolation> {
        let state = Box::new(reducer.state.clone());
        let revision = reducer.revision;
        let snapshot_sha256 = predecessor_digest(&state, revision)?;
        Ok(Self {
            state,
            revision,
            snapshot_sha256,
        })
    }

    pub(crate) fn restored(
        reducer: Reducer,
        snapshot_sha256: Sha256Digest,
    ) -> Result<Self, SnapshotViolation> {
        if reducer.state.revoked || reducer.state.revocation.is_some() {
            return Err(SnapshotViolation::InvalidState);
        }
        let proof = Self {
            state: Box::new(reducer.state),
            revision: reducer.revision,
            snapshot_sha256,
        };
        proof
            .matches_snapshot()
            .then_some(proof)
            .ok_or(SnapshotViolation::InvalidState)
    }

    pub(crate) fn replay(&self) -> Reducer {
        Reducer {
            state: (*self.state).clone(),
            revision: self.revision,
        }
    }

    pub(crate) fn state(&self) -> &NormalizedLifecycle {
        &self.state
    }

    pub(crate) const fn revision(&self) -> AuthorityRevision {
        self.revision
    }

    pub(crate) const fn snapshot_sha256(&self) -> Sha256Digest {
        self.snapshot_sha256
    }

    pub(crate) fn matches_snapshot(&self) -> bool {
        self.digest()
            .is_ok_and(|digest| digest == self.snapshot_sha256)
    }

    fn digest(&self) -> Result<Sha256Digest, crate::HookViolation> {
        predecessor_digest(&self.state, self.revision)
    }
}

fn predecessor_digest(
    state: &NormalizedLifecycle,
    revision: AuthorityRevision,
) -> Result<Sha256Digest, crate::HookViolation> {
    let document = crate::snapshot::document_from_state(state, revision);
    Sha256Digest::capture(&(PREDECESSOR_DIGEST_DOMAIN, document))
        .map_err(|_| crate::HookViolation::InvalidEnvelope)
}
