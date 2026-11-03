use super::super::constants::{
    LINUX_PTY_PROOF_OPEN_FD_SCAN_LIMIT, LINUX_PTY_PROOF_RETAINED_AUTHORITIES,
    LINUX_PTY_PROOF_RETAINED_FD_RESERVE, LINUX_PTY_PROOF_TRANSIENT_FD_BUDGET,
};
use super::linux_contract::CandidatePending;
use super::{CapabilityCandidates, ObservedProcess};
use crate::process::{LiveProcessIdentity, ProcessIdentity};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Clone)]
pub(super) struct MatchedCandidate {
    pub(super) live: LiveProcessIdentity,
    pub(super) durable: ProcessIdentity,
}

#[derive(Clone, Default)]
pub(super) struct LinuxScanState {
    pub(super) universe: Arc<[ProcessIdentity]>,
    pub(super) excluded: HashSet<ProcessIdentity>,
    pub(super) matched: HashMap<ProcessIdentity, MatchedCandidate>,
    pub(super) cursor: usize,
    pub(super) blocked: bool,
    pub(super) pending: Option<CandidatePending>,
}

impl LinuxScanState {
    pub(super) fn reconcile(&self, universe: Arc<[ProcessIdentity]>) -> Self {
        if self.universe.as_ref() == universe.as_ref() {
            return self.clone();
        }
        Self {
            universe,
            ..Self::default()
        }
    }
}

pub(super) fn materialize(
    state: &LinuxScanState,
    suppressed: &HashSet<ProcessIdentity>,
) -> CapabilityCandidates {
    let mut candidates = CapabilityCandidates::default();
    for (identity, matched) in &state.matched {
        if !suppressed.contains(identity) {
            record_match(&mut candidates, matched.clone());
        }
    }
    candidates
}

pub(super) fn record_match(candidates: &mut CapabilityCandidates, matched: MatchedCandidate) {
    let lineage = matched.live.lineage_id();
    candidates.observed.insert(lineage);
    candidates.stable.insert(lineage);
    if !candidates
        .identities
        .iter()
        .any(|process| process.live == matched.live)
    {
        candidates.identities.push(ObservedProcess {
            live: matched.live,
            durable: matched.durable,
        });
    }
}

pub(super) fn batch_end(start: usize, candidates: usize, limit: usize) -> usize {
    start.saturating_add(limit).min(candidates)
}

pub(super) fn retained_authority_limit(retained: usize) -> Option<usize> {
    let mut limit = libc::rlimit {
        rlim_cur: libc::rlim_t::default(),
        rlim_max: libc::rlim_t::default(),
    };
    if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit) } != 0 {
        return None;
    }
    authority_capacity(
        usize::try_from(limit.rlim_cur).unwrap_or(usize::MAX),
        open_descriptor_count()?,
        retained,
    )
}

fn authority_capacity(soft: usize, open: usize, retained: usize) -> Option<usize> {
    let unavailable = open
        .checked_add(LINUX_PTY_PROOF_RETAINED_FD_RESERVE)?
        .checked_add(LINUX_PTY_PROOF_TRANSIENT_FD_BUDGET)?;
    Some(
        retained
            .saturating_add(soft.saturating_sub(unavailable))
            .min(LINUX_PTY_PROOF_RETAINED_AUTHORITIES),
    )
}

fn open_descriptor_count() -> Option<usize> {
    let descriptors = std::fs::read_dir(super::super::procfs::file_descriptors_path(
        std::process::id(),
    ))
    .ok()?;
    let mut count = usize::default();
    for descriptor in descriptors {
        descriptor.ok()?;
        count = count.checked_add(1)?;
        if count > LINUX_PTY_PROOF_OPEN_FD_SCAN_LIMIT {
            return None;
        }
    }
    Some(count)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/scan/pty_session_scan_linux_state_tests.rs"]
mod tests;
