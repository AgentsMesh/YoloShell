fn select_holder_priority_with<Q, C, M>(
    current: &[ReleasedCandidate],
    state: &ReleasedScanState,
    mode: MacosGlobalScanMode,
    query: Q,
    capture_identity: C,
    generation_matches: M,
) -> priority::ReleasedPrioritySelection
where
    Q: FnOnce() -> super::super::macos_path_holders::PathHolderResult<HashSet<u32>>,
    C: FnMut(u32) -> Option<LiveProcessIdentity>,
    M: FnMut(&LiveProcessIdentity) -> Option<bool>,
{
    priority::released_priority_identities_with(
        current,
        mode,
        &HashSet::new(),
        &state.holder_hints,
        state.holder_observation,
        query,
        capture_identity,
        generation_matches,
    )
}

fn empty_holder_priority(
    current: &[ReleasedCandidate],
    state: &ReleasedScanState,
    mode: MacosGlobalScanMode,
) -> priority::ReleasedPrioritySelection {
    select_holder_priority_with(
        current,
        state,
        mode,
        || Ok(HashSet::new()),
        |_pid| None,
        |_identity| Some(true),
    )
}

fn advance_holder_selection<F>(
    current: Arc<[ReleasedCandidate]>,
    state: &ReleasedScanState,
    mode: MacosGlobalScanMode,
    selection: priority::ReleasedPrioritySelection,
    work_limit: usize,
    evaluate: F,
) -> CapabilityScanResult
where
    F: FnMut(ReleasedCandidate, &mut CapabilityScanBudget) -> anyhow::Result<CandidateEvaluation>,
{
    let priority::ReleasedPrioritySelection {
        identities,
        holder_identities,
        observed_holder_identities: _,
        holder_hints,
        observation,
    } = selection;
    let mut scheduled_state = state.clone();
    scheduled_state.holder_hints = holder_hints;
    let empty_holder_identities = HashSet::new();
    let conclusive_holder_identities = if mode == MacosGlobalScanMode::TransferableWriterReleased {
        &holder_identities
    } else {
        &empty_holder_identities
    };
    let mut result = advance::advance_with_holder_lifecycle(
        current,
        &scheduled_state,
        boundary::ReleasedPollBoundary::work_only(work_limit),
        ReleasedSweepPolicy::Frozen,
        &identities,
        conclusive_holder_identities,
        evaluate,
    )
    .unwrap();
    result.advance.next.released.holder_observation = observation;
    result
}
