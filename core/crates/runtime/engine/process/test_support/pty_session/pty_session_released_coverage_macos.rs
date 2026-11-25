impl ReleasedSweepCoverage {
    pub(super) fn snapshot_for_test(&self) -> (usize, Vec<LiveProcessIdentity>) {
        let mut pending = self.remaining.iter().copied().collect::<Vec<_>>();
        pending.sort_unstable_by_key(|identity| (identity.pid(), identity.lineage_id()));
        (
            self.round.as_ref().map_or(usize::default(), HashMap::len),
            pending,
        )
    }

    pub(super) fn queued_entries_for_test(&self) -> usize {
        self.queues.len()
    }
}

const MAX_DIAGNOSTIC_CANDIDATES: usize = 8;

impl super::ReleasedScanState {
    pub(in crate::process::pty_session_proof) fn sweep_diagnostic_for_test(&self) -> String {
        let coverage = &self.coverage;
        let pending = diagnostic_candidates(coverage.round.as_ref(), &coverage.remaining);
        let active_deferred = diagnostic_candidates(coverage.round.as_ref(), &coverage.deferred);
        let completed = coverage.completed.as_ref();
        let deferred = completed
            .map(|round| diagnostic_candidates(Some(&round.candidates), &round.deferred))
            .unwrap_or_default();
        let negative = completed
            .map(|round| {
                let identities = round
                    .candidates
                    .keys()
                    .filter(|identity| {
                        !round.deferred.contains(identity) && !self.matched.contains_key(identity)
                    })
                    .copied()
                    .collect();
                diagnostic_candidates(Some(&round.candidates), &identities)
            })
            .unwrap_or_default();
        let completed_count = completed.map_or(usize::default(), |round| round.candidates.len());
        let deferred_count = completed.map_or(usize::default(), |round| round.deferred.len());
        let negative_count = completed.map_or(usize::default(), |round| {
            round
                .candidates
                .keys()
                .filter(|identity| {
                    !round.deferred.contains(identity) && !self.matched.contains_key(identity)
                })
                .count()
        });
        format!(
            "active_pending_count={} active_pending={pending:?} active_deferred_count={} active_deferred={active_deferred:?} completed_count={completed_count} completed_deferred_count={deferred_count} completed_deferred={deferred:?} completed_negative_count={negative_count} completed_negative={negative:?}",
            coverage.remaining.len(),
            coverage.deferred.len(),
        )
    }
}

fn diagnostic_candidates(
    candidates: Option<&HashMap<LiveProcessIdentity, ReleasedCandidate>>,
    identities: &HashSet<LiveProcessIdentity>,
) -> Vec<String> {
    let mut ordered = identities.iter().copied().collect::<Vec<_>>();
    ordered.sort_unstable_by_key(|identity| (identity.pid(), identity.lineage_id()));
    ordered
        .into_iter()
        .take(MAX_DIAGNOSTIC_CANDIDATES)
        .map(|identity| {
            let candidate = candidates.and_then(|values| values.get(&identity));
            let generation =
                candidate.map(|entry| (entry.durable.start_time, entry.parent_lineage));
            format!(
                "pid={} lineage={} generation={generation:?}",
                identity.pid(),
                identity.lineage_id()
            )
        })
        .collect()
}
