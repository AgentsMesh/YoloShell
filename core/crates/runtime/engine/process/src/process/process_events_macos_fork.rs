use super::{EVENT_BATCH_CAPACITY, EVENT_DETAIL, MacosProcessEvents, empty_event};
use crate::process::LiveProcessIdentity;
use anyhow::{Context, Result, ensure};
use std::collections::{HashMap, HashSet};

const FORK_SOURCE_DETAIL: &str = "macOS fork event source is not registered";
const FORK_EVENT_VERSION_INCREMENT: u64 = 1;
const FORK_EVENT_VERSION_EXHAUSTED_DETAIL: &str = "macOS fork event version is exhausted";

impl MacosProcessEvents {
    pub(in crate::process) fn changed(&mut self) -> Result<bool> {
        #[cfg(any(test, feature = "runtime-component-tests"))]
        self.test_faults.check_read()?;
        let mut changed = false;
        loop {
            let mut events = vec![empty_event(); EVENT_BATCH_CAPACITY];
            let count = self.read(&mut events)?;
            changed |= count != usize::default();
            ensure!(
                events[..count]
                    .iter()
                    .all(|event| event.filter == libc::EVFILT_PROC
                        && event.flags & libc::EV_ERROR == u16::default()),
                EVENT_DETAIL
            );
            for event in &events[..count] {
                let pid = u32::try_from(event.ident)?;
                if event.fflags & libc::NOTE_FORK != 0 {
                    let source = self
                        .registered
                        .get(&pid)
                        .map(LiveProcessIdentity::lineage_id)
                        .context(FORK_SOURCE_DETAIL)?;
                    self.observe_fork(source)?;
                }
                if event.fflags & libc::NOTE_EXIT != 0
                    && let Some(source) = self.registered.get(&pid).copied()
                {
                    self.root_exited |= source == self.root;
                    if self.unresolved_forks.contains(&source.lineage_id()) {
                        self.gone_fork_sources.insert(source.lineage_id());
                    }
                }
            }
            if count < EVENT_BATCH_CAPACITY {
                return Ok(changed);
            }
        }
    }

    pub(in crate::process) fn has_unresolved_fork(&self) -> bool {
        !self.unresolved_forks.is_empty()
    }

    pub(in crate::process) fn unresolved_fork_versions(&self) -> HashMap<u64, u64> {
        self.fork_source_versions.clone()
    }

    pub(super) fn observe_fork(&mut self, source: u64) -> Result<()> {
        self.fork_event_version = self
            .fork_event_version
            .checked_add(FORK_EVENT_VERSION_INCREMENT)
            .context(FORK_EVENT_VERSION_EXHAUSTED_DETAIL)?;
        self.unresolved_forks.insert(source);
        self.fork_source_versions
            .insert(source, self.fork_event_version);
        Ok(())
    }

    pub(in crate::process) fn exact_gone_fork_source(&self, identity: LiveProcessIdentity) -> bool {
        self.gone_fork_sources.contains(&identity.lineage_id())
    }

    pub(in crate::process) fn exact_root_exited(&self, identity: LiveProcessIdentity) -> bool {
        self.root_exited && self.root == identity
    }

    pub(in crate::process) fn confirm_root_exit(&mut self, identity: LiveProcessIdentity) {
        if self.root == identity {
            self.root_exited = true;
        }
    }

    pub(in crate::process) fn generation_can_still_fork(
        &self,
        identity: LiveProcessIdentity,
    ) -> Result<bool> {
        let source = identity.lineage_id();
        if self.exact_root_exited(identity) || self.gone_fork_sources.contains(&source) {
            return Ok(false);
        }
        identity.try_matches()
    }

    pub(in crate::process) fn mark_generation_gone(&mut self, identity: LiveProcessIdentity) {
        let source = identity.lineage_id();
        if self.unresolved_forks.contains(&source) {
            self.gone_fork_sources.insert(source);
        }
    }

    pub(in crate::process) fn exact_closure_isolated(
        &self,
        closure: &[super::LiveProcessIdentity],
    ) -> bool {
        if self.unresolved_forks.is_empty() {
            return false;
        }
        self.unresolved_forks.iter().all(|source| {
            let in_closure = closure
                .iter()
                .any(|identity| identity.lineage_id() == *source);
            let registered = self
                .registered
                .values()
                .any(|identity| identity.lineage_id() == *source);
            self.gone_fork_sources.contains(source) && !in_closure && !registered
        })
    }

    pub(in crate::process) fn exact_containment_snapshot(
        &self,
        allowed: &HashMap<u64, u64>,
    ) -> Option<(HashMap<u64, u64>, u64)> {
        (!self.unresolved_forks.is_empty()
            && self.unresolved_forks.len() == allowed.len()
            && self
                .unresolved_forks
                .iter()
                .all(|source| self.fork_source_versions.get(source) == allowed.get(source))
            && self.exact_closure_isolated(&[]))
        .then(|| (allowed.clone(), self.fork_event_version))
    }

    pub(in crate::process) fn acknowledge_exact_containment(
        &mut self,
        closure: &[super::LiveProcessIdentity],
        allowed: &HashMap<u64, u64>,
        fork_event_version: u64,
    ) -> bool {
        self.fork_event_version == fork_event_version
            && self.exact_closure_isolated(closure)
            && self.fork_source_versions == *allowed
            && {
                self.unresolved_forks.clear();
                self.fork_source_versions.clear();
                self.gone_fork_sources.clear();
                true
            }
    }

    pub(in crate::process) fn acknowledge_frozen_forks(
        &mut self,
        frozen_lineages: &HashSet<u64>,
    ) -> bool {
        let before = self.unresolved_forks.len();
        self.unresolved_forks
            .retain(|source| !frozen_lineages.contains(source));
        if self.unresolved_forks.len() == before {
            return false;
        }
        self.fork_source_versions
            .retain(|source, _version| self.unresolved_forks.contains(source));
        self.gone_fork_sources
            .retain(|source| self.unresolved_forks.contains(source));
        true
    }

    pub(in crate::process) fn acknowledge_root_exit_discovery_epoch(&mut self) {
        self.unresolved_forks.clear();
        self.fork_source_versions.clear();
        self.gone_fork_sources.clear();
    }
}
