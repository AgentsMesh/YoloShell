use crate::agent::{AgentEvidence, AgentKind, ObservedProcess};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CandidateAuthority {
    Contextual,
    ProcessAssociated,
}

pub(super) struct Candidate {
    pub kind: AgentKind,
    pub process: ObservedProcess,
    pub evidence: AgentEvidence,
    authority: CandidateAuthority,
}

#[derive(Default)]
pub(super) struct ProcessOutcome {
    pub candidates: Vec<Candidate>,
    pub invalidates_binding: bool,
}

pub(super) fn retain_strongest(candidates: &mut Vec<Candidate>) {
    if candidates
        .iter()
        .any(|candidate| candidate.authority == CandidateAuthority::ProcessAssociated)
    {
        candidates.retain(|candidate| candidate.authority == CandidateAuthority::ProcessAssociated);
    }
}

pub(super) fn unique_outcome(
    kind: AgentKind,
    process: &ObservedProcess,
    mut evidence: Vec<AgentEvidence>,
    invalidates_binding: bool,
    authority: CandidateAuthority,
) -> ProcessOutcome {
    let unique = evidence.len() == 1;
    let candidates = if unique {
        vec![Candidate {
            kind,
            process: process.clone(),
            evidence: evidence
                .pop()
                .expect(crate::agent::constants::validation::SINGLE_EVIDENCE),
            authority,
        }]
    } else {
        Vec::new()
    };
    ProcessOutcome {
        candidates,
        invalidates_binding: invalidates_binding && !unique,
    }
}
