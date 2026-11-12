use super::super::{
    CapabilityCandidates, CapabilityScanAdvance, CapabilityScanResult, CapabilityScanStatus,
};
use super::ReleasedScanState;
use super::universe::ReleasedCandidate;
use crate::process::LiveProcessIdentity;
use crate::process::captured_lineage::ObservedProcess;
use std::collections::HashMap;

pub(super) fn materialize_evidence(
    matched: &HashMap<LiveProcessIdentity, ReleasedCandidate>,
) -> CapabilityCandidates {
    let mut evidence = CapabilityCandidates {
        status: CapabilityScanStatus::Pending,
        ..CapabilityCandidates::default()
    };
    let mut retained = matched.values().copied().collect::<Vec<_>>();
    retained.sort_by_key(|candidate| (candidate.live.pid(), candidate.live.lineage_id()));
    for candidate in retained {
        record_match(&mut evidence, candidate);
    }
    evidence
}

pub(super) fn record_match(evidence: &mut CapabilityCandidates, candidate: ReleasedCandidate) {
    // The root is already an exact session anchor. Its positive match is retained
    // as kill authority, but must not be republished as an escaped descendant.
    if candidate.capability_root {
        return;
    }
    let lineage = candidate.live.lineage_id();
    evidence.observed.insert(lineage);
    if candidate.stable {
        evidence.stable.insert(lineage);
    }
    evidence.identities.push(ObservedProcess {
        live: candidate.live,
        durable: candidate.durable,
    });
}

pub(super) fn pending(
    evidence: CapabilityCandidates,
    next: ReleasedScanState,
    scheduling_complete: bool,
    coverage_complete: bool,
    batch_consumed: bool,
    evidence_observed: bool,
) -> CapabilityScanResult {
    result(
        evidence,
        next,
        coverage_status(scheduling_complete),
        coverage_status(coverage_complete),
        batch_consumed,
        evidence_observed,
    )
}

pub(super) fn completed_round(
    evidence: CapabilityCandidates,
    next: ReleasedScanState,
    scheduling_complete: bool,
    coverage_complete: bool,
    batch_consumed: bool,
    evidence_observed: bool,
) -> CapabilityScanResult {
    result(
        evidence,
        next,
        coverage_status(scheduling_complete),
        coverage_status(coverage_complete),
        batch_consumed,
        evidence_observed,
    )
}

fn coverage_status(complete: bool) -> CapabilityScanStatus {
    if complete {
        CapabilityScanStatus::Complete
    } else {
        CapabilityScanStatus::Pending
    }
}

fn result(
    candidates: CapabilityCandidates,
    next: ReleasedScanState,
    scheduling_status: CapabilityScanStatus,
    coverage_status: CapabilityScanStatus,
    batch_consumed: bool,
    evidence_observed: bool,
) -> CapabilityScanResult {
    CapabilityScanResult {
        candidates,
        advance: CapabilityScanAdvance::released(next)
            .with_consumed_batch(batch_consumed)
            .with_observed_evidence(evidence_observed),
        scheduling_status,
        coverage_status,
    }
}
