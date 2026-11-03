pub(super) fn advance_scan<F>(
    current: Arc<[ReleasedCandidate]>,
    state: &ReleasedScanState,
    work_limit: usize,
    evaluate: F,
) -> Result<super::CapabilityScanResult>
where
    F: FnMut(ReleasedCandidate, &mut CapabilityScanBudget) -> Result<CandidateEvaluation>,
{
    advance_with_policy(
        current,
        state,
        work_limit,
        ReleasedSweepPolicy::Live,
        evaluate,
    )
}

pub(super) fn advance_with_policy<F>(
    current: Arc<[ReleasedCandidate]>,
    state: &ReleasedScanState,
    work_limit: usize,
    policy: ReleasedSweepPolicy,
    evaluate: F,
) -> Result<super::CapabilityScanResult>
where
    F: FnMut(ReleasedCandidate, &mut CapabilityScanBudget) -> Result<CandidateEvaluation>,
{
    advance_with_boundary(
        current,
        state,
        ReleasedPollBoundary::work_only(work_limit),
        policy,
        evaluate,
    )
}

pub(super) fn advance_with_boundary<F>(
    current: Arc<[ReleasedCandidate]>,
    state: &ReleasedScanState,
    boundary: ReleasedPollBoundary,
    policy: ReleasedSweepPolicy,
    evaluate: F,
) -> Result<super::CapabilityScanResult>
where
    F: FnMut(ReleasedCandidate, &mut CapabilityScanBudget) -> Result<CandidateEvaluation>,
{
    advance_with_priority_boundary(current, state, boundary, policy, &HashSet::new(), evaluate)
}

pub(super) fn advance_with_priority_boundary(
    current: Arc<[ReleasedCandidate]>,
    state: &ReleasedScanState,
    boundary: ReleasedPollBoundary,
    policy: ReleasedSweepPolicy,
    exact_priorities: &HashSet<LiveProcessIdentity>,
    evaluate: impl FnMut(ReleasedCandidate, &mut CapabilityScanBudget) -> EvaluationResult,
) -> Result<super::CapabilityScanResult> {
    advance_with_holder_lifecycle(
        current,
        state,
        boundary,
        policy,
        exact_priorities,
        &HashSet::new(),
        evaluate,
    )
}
