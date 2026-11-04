use super::CapturedProcessTree;
use crate::process::discovery_custody_budget::PRE_SIGNAL_REFRESH_EPOCHS;
use crate::process::{LiveProcessIdentity, ProcessIdentity, captured_targets};
use anyhow::Result;

#[derive(Clone, Copy)]
enum LiveTargetScope {
    EntireTree,
    Descendants,
}

impl LiveTargetScope {
    fn includes(self, identity: &LiveProcessIdentity, root: &LiveProcessIdentity) -> bool {
        match self {
            Self::EntireTree => true,
            Self::Descendants => identity != root,
        }
    }
}

impl CapturedProcessTree {
    pub fn live_identities(&self) -> Result<Vec<ProcessIdentity>> {
        self.live_targets(LiveTargetScope::EntireTree)
            .map(captured_targets::durable)
    }

    pub fn live_descendants(&self) -> Result<Vec<ProcessIdentity>> {
        self.live_targets(LiveTargetScope::Descendants)
            .map(captured_targets::durable)
    }

    pub fn request_graceful_termination(&mut self) -> Result<Vec<ProcessIdentity>> {
        self.destructive_authority()?;
        let refresh_failure = refresh_before_signal(self);
        let targets = self.captured_targets();
        let authority = self.destructive_authority()?;
        captured_targets::combine(
            refresh_failure,
            captured_targets::signal(authority, &targets, libc::SIGTERM),
        )?;
        Ok(captured_targets::durable(targets))
    }

    pub(crate) fn request_force_termination(&mut self) -> Result<Vec<ProcessIdentity>> {
        self.destructive_authority()?;
        let refresh_failure = refresh_before_signal(self);
        let targets = self.captured_targets();
        let authority = self.destructive_authority()?;
        let signals = captured_targets::signal_sigkill(authority, &targets);
        if refresh_failure.is_none()
            && let Ok(Some(delivery)) = &signals
        {
            self.commit_exact_lineage_containment(delivery);
        }
        captured_targets::combine(refresh_failure, signals)?;
        Ok(captured_targets::durable(targets))
    }

    pub(crate) fn observe_live_identities(&mut self) -> Result<Vec<ProcessIdentity>> {
        self.refresh()?;
        self.live_identities()
    }

    pub fn request_graceful_descendant_termination(&mut self) -> Result<Vec<ProcessIdentity>> {
        self.destructive_authority()?;
        let refresh_failure = refresh_before_signal(self);
        let targets = self.captured_descendant_targets();
        let authority = self.destructive_authority()?;
        let signals = captured_targets::signal(authority, &targets, libc::SIGTERM);
        captured_targets::combine(refresh_failure, signals)?;
        Ok(captured_targets::durable(targets))
    }

    #[cfg(test)]
    pub fn terminate(&mut self, signal: i32) -> Result<Vec<ProcessIdentity>> {
        let refresh_failure = refresh_before_signal(self);
        let targets = self.captured_targets();
        captured_targets::combine(
            refresh_failure,
            captured_targets::signal_for_test(&targets, signal),
        )?;
        Ok(captured_targets::durable(targets))
    }

    #[cfg(test)]
    pub fn terminate_captured(&self, signal: i32) -> Result<Vec<ProcessIdentity>> {
        let targets = self.captured_targets();
        captured_targets::signal_for_test(&targets, signal)?;
        Ok(captured_targets::durable(targets))
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn terminate_descendants(&mut self, signal: i32) -> Result<Vec<ProcessIdentity>> {
        let refresh_failure = refresh_before_signal(self);
        let targets = self.signal_descendants(signal);
        captured_targets::combine(refresh_failure, targets)
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn signal_descendants(&self, signal: i32) -> Result<Vec<ProcessIdentity>> {
        let targets = self.captured_descendant_targets();
        captured_targets::signal_for_test(&targets, signal)?;
        Ok(captured_targets::durable(targets))
    }

    fn live_targets(
        &self,
        scope: LiveTargetScope,
    ) -> Result<Vec<(LiveProcessIdentity, ProcessIdentity)>> {
        let root = self.scope.live_root();
        let mut identities = Vec::new();
        for (live, durable) in &self.captured {
            if live == &root && self.root_exit_is_proven() {
                continue;
            }
            if !self.boundary.contains(live) && scope.includes(live, &root) && live.try_matches()? {
                identities.push((live.clone(), *durable));
            }
        }
        Ok(self.sorted(identities.into_iter()))
    }

    fn captured_descendant_targets(&self) -> Vec<(LiveProcessIdentity, ProcessIdentity)> {
        let root = self.scope.live_root();
        self.sorted(self.captured.iter().filter_map(|(live, durable)| {
            (live != &root && !self.boundary.contains(live)).then_some((live.clone(), *durable))
        }))
    }

    pub(super) fn captured_targets(&self) -> Vec<(LiveProcessIdentity, ProcessIdentity)> {
        let mut targets = self.sorted(
            self.captured
                .iter()
                .map(|(live, durable)| (live.clone(), *durable)),
        );
        targets.retain(|(identity, _durable)| !self.boundary.contains(identity));
        if self.root_exit_is_proven() {
            let root = self.scope.live_root();
            targets.retain(|(identity, _durable)| identity != &root);
        }
        targets
    }
}

fn refresh_before_signal(tree: &mut CapturedProcessTree) -> Option<anyhow::Error> {
    (0..PRE_SIGNAL_REFRESH_EPOCHS).find_map(|_| tree.refresh().err())
}
