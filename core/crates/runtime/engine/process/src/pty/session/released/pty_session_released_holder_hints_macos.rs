use crate::process::LiveProcessIdentity;
use std::collections::HashSet;

#[derive(Clone, Default)]
pub(super) struct ReleasedHolderHints {
    identities: HashSet<LiveProcessIdentity>,
}

impl ReleasedHolderHints {
    pub(super) fn reconcile_with<F>(&self, mut generation_matches: F) -> Self
    where
        F: FnMut(&LiveProcessIdentity) -> Option<bool>,
    {
        let mut next = self.clone();
        next.identities
            .retain(|identity| generation_matches(identity) != Some(false));
        next
    }

    pub(super) fn extend<I>(&mut self, identities: I)
    where
        I: IntoIterator<Item = LiveProcessIdentity>,
    {
        self.identities.extend(identities);
    }

    pub(super) fn remove(&mut self, identity: &LiveProcessIdentity) {
        self.identities.remove(identity);
    }

    pub(super) fn priorities_for(
        &self,
        current: &[super::universe::ReleasedCandidate],
    ) -> HashSet<LiveProcessIdentity> {
        current
            .iter()
            .map(|candidate| candidate.live)
            .filter(|identity| self.identities.contains(identity))
            .collect()
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub(super) fn identities(&self) -> &HashSet<LiveProcessIdentity> {
        &self.identities
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub(super) fn contains(&self, identity: &LiveProcessIdentity) -> bool {
        self.identities.contains(identity)
    }
}
