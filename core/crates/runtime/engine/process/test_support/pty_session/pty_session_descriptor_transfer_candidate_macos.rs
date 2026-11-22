use super::*;

pub(super) fn released_candidate(
    identity: ProcessIdentity,
    environment_eligible: bool,
) -> ReleasedCandidate {
    ReleasedCandidate {
        live: LiveProcessIdentity::capture(identity).unwrap().unwrap(),
        durable: identity,
        parent_lineage: u64::default(),
        stable: true,
        capability_root: false,
        environment_eligible,
    }
}
