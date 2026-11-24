use super::CapturedProcessTree;
use super::frozen_authority::FrozenClosureAuthority;
use crate::process::{LiveProcessIdentity, captured_targets};
use anyhow::Result;

include!("captured_tree_freeze_candidate.rs");

pub(super) fn observe(
    tree: &mut CapturedProcessTree,
    destructive_started: &mut bool,
) -> Result<Option<FreezeClosureCandidate>> {
    #[cfg(target_os = "linux")]
    if tree.linux_containment_available() {
        *destructive_started = true;
        return tree
            .linux_frozen_closure(true)
            .map(|closure| closure.map(FreezeClosureCandidate::Ordinary));
    }
    if let Some(candidate) = exact_positive_stopped_candidate(tree)? {
        return Ok(Some(candidate));
    }
    if !tree.discovery_epoch.coverage_is_complete() {
        refresh_freeze_coverage(tree)?;
    }
    if let Some(candidate) = exact_stopped_candidate(tree)? {
        return Ok(Some(candidate));
    }
    if let std::ops::ControlFlow::Break(candidate) = consume_covered_stopped_closure(tree)? {
        return Ok(candidate);
    }
    tree.refresh()?;
    let targets = tree.captured_targets();
    if targets.is_empty() {
        return stopped_closure_candidate(tree, targets);
    }
    let authority = tree.destructive_authority()?;
    *destructive_started = true;
    captured_targets::signal(authority, &targets, libc::SIGSTOP)?;
    stopped_closure_candidate(tree, targets)
}

include!("captured_tree_freeze_coverage.rs");

pub(super) fn inspect(tree: &mut CapturedProcessTree) -> Result<Option<FreezeClosureCandidate>> {
    #[cfg(target_os = "linux")]
    if tree.linux_containment_available() {
        return tree
            .linux_frozen_closure(false)
            .map(|closure| closure.map(FreezeClosureCandidate::Ordinary));
    }
    if let Some(candidate) = exact_stopped_candidate(tree)? {
        return Ok(Some(candidate));
    }
    if let std::ops::ControlFlow::Break(candidate) = consume_covered_stopped_closure(tree)? {
        return Ok(candidate);
    }
    tree.refresh()?;
    let targets = tree.captured_targets();
    stopped_closure_candidate(tree, targets)
}

pub(super) fn inspect_with_kernel_eof(
    tree: &mut CapturedProcessTree,
) -> Result<Option<FreezeClosureCandidate>> {
    if let Some(candidate) = exact_stopped_candidate(tree)? {
        return Ok(Some(candidate));
    }
    if let std::ops::ControlFlow::Break(candidate) = consume_covered_stopped_closure(tree)? {
        return Ok(candidate);
    }
    tree.refresh_for_kernel_eof()?;
    let targets = tree.captured_targets();
    stopped_closure_candidate(tree, targets)
}

include!("captured_tree_freeze_fresh.rs");

fn stopped_closure_candidate(
    tree: &mut CapturedProcessTree,
    targets: Vec<(LiveProcessIdentity, crate::process::ProcessIdentity)>,
) -> Result<Option<FreezeClosureCandidate>> {
    if targets.is_empty() {
        if restart_after_frozen_fork_if_required(tree, &[]) {
            return Ok(None);
        }
        return Ok(tree
            .frozen_closure_is_settled(FrozenClosureAuthority::OutputFencedEmpty)
            .then(|| FreezeClosureCandidate::Ordinary(Vec::new())));
    }
    let Some(authority) = tree.captured_stopped_closure_authority(&targets)? else {
        return Ok(None);
    };
    if restart_after_frozen_fork_if_required(tree, authority.fork_sources()) {
        return Ok(None);
    }
    if tree.exact_stopped_closure_can_kill(&authority) {
        return Ok(Some(FreezeClosureCandidate::Exact(
            authority.into_closure(),
        )));
    }
    if tree.incremental_stopped_closure_can_kill(&authority) {
        return Ok(Some(FreezeClosureCandidate::IncrementalExact(
            authority.into_closure(),
        )));
    }
    if tree.kernel_eof_stopped_closure_can_kill(&authority) {
        return Ok(Some(FreezeClosureCandidate::KernelEof(
            authority.into_closure(),
        )));
    }
    if tree.acknowledge_frozen_forks(authority.fork_sources())
        || !tree.frozen_closure_is_settled(FrozenClosureAuthority::CapturedStopped(&authority))
    {
        return Ok(None);
    }
    Ok(Some(FreezeClosureCandidate::Ordinary(
        authority.into_closure(),
    )))
}

pub(super) fn exact_stopped_candidate(
    tree: &mut CapturedProcessTree,
) -> Result<Option<FreezeClosureCandidate>> {
    stopped_candidate(tree, StoppedCandidateScope::AllAuthorities)
}

fn exact_positive_stopped_candidate(
    tree: &mut CapturedProcessTree,
) -> Result<Option<FreezeClosureCandidate>> {
    stopped_candidate(tree, StoppedCandidateScope::ExactPositiveOnly)
}

fn stopped_candidate(
    tree: &mut CapturedProcessTree,
    scope: StoppedCandidateScope,
) -> Result<Option<FreezeClosureCandidate>> {
    let targets = tree.captured_targets();
    if targets.is_empty() {
        return match scope {
            StoppedCandidateScope::ExactPositiveOnly => Ok(None),
            StoppedCandidateScope::AllAuthorities => Ok(tree
                .frozen_closure_is_settled(FrozenClosureAuthority::OutputFencedEmpty)
                .then(|| FreezeClosureCandidate::Ordinary(Vec::new()))),
        };
    }
    let Some(authority) = tree.captured_stopped_closure_authority(&targets)? else {
        return Ok(None);
    };
    if tree.exact_stopped_closure_can_kill(&authority) {
        return Ok(Some(FreezeClosureCandidate::Exact(
            authority.into_closure(),
        )));
    }
    if tree.incremental_stopped_closure_can_kill(&authority) {
        return Ok(Some(FreezeClosureCandidate::IncrementalExact(
            authority.into_closure(),
        )));
    }
    match scope {
        StoppedCandidateScope::ExactPositiveOnly => Ok(None),
        StoppedCandidateScope::AllAuthorities => Ok(tree
            .kernel_eof_stopped_closure_can_kill(&authority)
            .then(|| FreezeClosureCandidate::KernelEof(authority.into_closure()))),
    }
}
#[cfg(any(test, feature = "runtime-component-tests"))]
fn exact_stopped_closure(
    tree: &mut CapturedProcessTree,
) -> Result<Option<Vec<LiveProcessIdentity>>> {
    Ok(exact_stopped_candidate(tree)?.map(FreezeClosureCandidate::into_closure))
}
#[cfg(any(test, feature = "runtime-component-tests"))]
fn stopped_closure(
    tree: &mut CapturedProcessTree,
    targets: Vec<(LiveProcessIdentity, crate::process::ProcessIdentity)>,
) -> Result<Option<Vec<LiveProcessIdentity>>> {
    Ok(stopped_closure_candidate(tree, targets)?.map(FreezeClosureCandidate::into_closure))
}
#[cfg(any(test, feature = "runtime-component-tests"))]
impl FreezeClosureCandidate {
    pub(super) fn into_closure(self) -> Vec<LiveProcessIdentity> {
        match self {
            Self::Exact(closure)
            | Self::IncrementalExact(closure)
            | Self::Ordinary(closure)
            | Self::KernelEof(closure) => closure,
        }
    }
}

include!("captured_tree_freeze_covered.rs");

#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
include!("../../../../integration_tests/captured/tree/captured_tree_freeze_test_wiring.rs");
