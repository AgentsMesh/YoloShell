impl ReleasedSweepCoverage {
    fn begin_new_round(
        &mut self,
        candidates: HashMap<LiveProcessIdentity, ReleasedCandidate>,
        ordered: &[ReleasedCandidate],
        policy: ReleasedSweepPolicy,
        exact_priorities: &HashSet<LiveProcessIdentity>,
    ) -> Result<(bool, bool, bool, bool)> {
        #[cfg(all(feature = "adversarial-test-hooks", not(test)))]
        if policy == ReleasedSweepPolicy::Frozen {
            super::super::macos_scan_diagnostic::record_frozen_round_started();
        }
        let retry = self.frozen_retry_round(&candidates, ordered, policy, exact_priorities);
        let evidence_observed = self.completed.as_ref().is_none_or(|completed| {
            completed.policy != policy || completed.candidates != candidates
        });
        self.discard_round();
        self.policy = policy;
        match retry {
            Some((round, scheduled)) => {
                self.start_retry_round(round, &scheduled, exact_priorities)?
            }
            None => self.start_round(candidates, ordered, exact_priorities)?,
        }
        let (scheduled, conclusive) = self.complete_if_empty();
        Ok((scheduled, conclusive, false, evidence_observed))
    }

    fn frozen_retry_round(
        &self,
        current: &HashMap<LiveProcessIdentity, ReleasedCandidate>,
        ordered: &[ReleasedCandidate],
        policy: ReleasedSweepPolicy,
        exact_priorities: &HashSet<LiveProcessIdentity>,
    ) -> Option<(
        HashMap<LiveProcessIdentity, ReleasedCandidate>,
        Vec<ReleasedCandidate>,
    )> {
        let completed = self.completed.as_ref()?;
        if policy != ReleasedSweepPolicy::Frozen
            || completed.policy != policy
            || completed.deferred.is_empty()
        {
            return None;
        }
        let mut round = HashMap::new();
        let mut retry = HashSet::new();
        for candidate in ordered.iter().copied() {
            let previous = completed.candidates.get(&candidate.live);
            let exact = exact_priorities.contains(&candidate.live);
            if previous.is_none() && !exact {
                continue;
            }
            round.insert(candidate.live, candidate);
            if exact || completed.deferred.contains(&candidate.live) || previous != Some(&candidate)
            {
                retry.insert(candidate.live);
            }
        }
        debug_assert!(round.keys().all(|identity| current.contains_key(identity)));
        let scheduled = ordered
            .iter()
            .copied()
            .filter(|candidate| retry.contains(&candidate.live))
            .collect();
        Some((round, scheduled))
    }

    fn start_retry_round(
        &mut self,
        candidates: HashMap<LiveProcessIdentity, ReleasedCandidate>,
        scheduled: &[ReleasedCandidate],
        exact_priorities: &HashSet<LiveProcessIdentity>,
    ) -> Result<()> {
        self.queues.start(scheduled)?;
        self.remaining = scheduled.iter().map(|candidate| candidate.live).collect();
        self.deferred.clear();
        self.prioritized = self
            .remaining
            .iter()
            .filter(|identity| exact_priorities.contains(identity))
            .copied()
            .collect();
        self.round = Some(candidates);
        Ok(())
    }

    pub(super) fn has_active_frozen_round(&self) -> bool {
        self.policy == ReleasedSweepPolicy::Frozen && self.round.is_some()
    }

    fn reconcile_round(
        &mut self,
        candidates: HashMap<LiveProcessIdentity, ReleasedCandidate>,
        ordered: &[ReleasedCandidate],
        exact_priorities: &HashSet<LiveProcessIdentity>,
    ) -> Result<(bool, HashSet<LiveProcessIdentity>)> {
        let identities = candidates.keys().copied().collect::<HashSet<_>>();
        let changed = self.retain_current(&identities);
        self.prioritized
            .retain(|identity| exact_priorities.contains(identity));
        let previous = self.round.as_mut().expect(ROUND_PRESENT_DETAIL);
        if self.policy == ReleasedSweepPolicy::Frozen {
            let mut additions = Vec::new();
            let mut exact_added = false;
            for candidate in ordered.iter().copied() {
                if let Some(retained) = previous.get_mut(&candidate.live) {
                    if *retained != candidate && self.remaining.contains(&candidate.live) {
                        additions.push(candidate);
                    }
                    *retained = candidate;
                } else if exact_priorities.contains(&candidate.live) {
                    previous.insert(candidate.live, candidate);
                    self.remaining.insert(candidate.live);
                    self.prioritized.insert(candidate.live);
                    additions.push(candidate);
                    exact_added = true;
                }
            }
            self.queues
                .enqueue_priority(&additions, previous, &self.remaining)?;
            let scheduled = additions.iter().map(|candidate| candidate.live).collect();
            return Ok((changed || exact_added, scheduled));
        }
        let mut added_or_changed = false;
        let mut additions = Vec::new();
        for candidate in ordered.iter().copied() {
            if previous.get(&candidate.live) == Some(&candidate) {
                continue;
            }
            previous.insert(candidate.live, candidate);
            self.remaining.insert(candidate.live);
            additions.push(candidate);
            added_or_changed = true;
        }
        self.queues
            .enqueue_priority(&additions, previous, &self.remaining)?;
        let scheduled = additions.iter().map(|candidate| candidate.live).collect();
        Ok((changed || added_or_changed, scheduled))
    }

    fn retain_current(&mut self, identities: &HashSet<LiveProcessIdentity>) -> bool {
        let round = self.round.as_mut().expect(ROUND_PRESENT_DETAIL);
        let before = round.len();
        round.retain(|identity, _candidate| identities.contains(identity));
        self.remaining
            .retain(|identity| identities.contains(identity));
        self.deferred
            .retain(|identity| identities.contains(identity));
        self.prioritized
            .retain(|identity| identities.contains(identity));
        before != round.len()
    }

    fn complete_if_empty(&mut self) -> (bool, bool) {
        if !self.remaining.is_empty() {
            return (false, false);
        }
        let conclusive = self.deferred.is_empty();
        #[cfg(all(feature = "adversarial-test-hooks", not(test)))]
        if self.policy == ReleasedSweepPolicy::Frozen {
            super::super::macos_scan_diagnostic::record_frozen_round_completed(conclusive);
        }
        self.finish_round();
        (true, conclusive)
    }
}
