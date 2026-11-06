use super::super::termination::RetainedExclusion;
use super::super::{CapturedProcessTree, ExactClosureKill};

#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub(super) enum PostKillObservation {
    Pending,
    Refreeze,
    ExactEmpty,
    Ordinary,
}

pub(super) struct PostKillExactClosure {
    phase: PostKillPhase,
}

enum PostKillPhase {
    Exact {
        kill: ExactClosureKill,
        remaining_polls: usize,
        yielded_after_exit: bool,
        retained_exclusion: RetainedExclusion,
    },
    Ordinary,
}

impl PostKillExactClosure {
    pub(super) fn exact(kill: ExactClosureKill) -> Self {
        let retained_exclusion = kill.retained_exclusion();
        Self {
            phase: PostKillPhase::Exact {
                kill,
                remaining_polls: crate::process::constants::POST_KILL_EXACT_GENERATION_POLL_LIMIT,
                yielded_after_exit: false,
                retained_exclusion,
            },
        }
    }

    pub(super) fn ordinary() -> Self {
        Self {
            phase: PostKillPhase::Ordinary,
        }
    }

    pub(super) fn observe(
        &mut self,
        tree: &mut CapturedProcessTree,
    ) -> anyhow::Result<PostKillObservation> {
        if matches!(self.phase, PostKillPhase::Ordinary) {
            return Ok(PostKillObservation::Ordinary);
        }
        #[cfg(target_os = "macos")]
        tree.reconcile_retained_root_exit()?;
        tree.refresh()?;
        if let Some(_generation_replacement) = self.unexpected_target_generation_replacement(tree) {
            self.revoke_retained_exclusion();
            #[cfg(target_os = "macos")]
            if !_generation_replacement
                && tree.events.is_gated()
                && !tree.events.inventory_publication_is_current()
            {
                return Ok(PostKillObservation::Pending);
            }
            return Ok(PostKillObservation::Refreeze);
        }
        if let Some(observation) = self.observe_exact_generations(tree)? {
            return Ok(observation);
        }
        if !tree.captured_targets().is_empty() {
            self.revoke_retained_exclusion();
            return Ok(PostKillObservation::Refreeze);
        }
        Ok(match self.observe_retained_exclusion(tree) {
            RetainedExclusion::Ready => PostKillObservation::ExactEmpty,
            RetainedExclusion::Pending => PostKillObservation::Pending,
            RetainedExclusion::Revoked => observe_rebuilt_exclusion(tree),
        })
    }

    fn unexpected_target_generation_replacement(&self, tree: &CapturedProcessTree) -> Option<bool> {
        let PostKillPhase::Exact { kill, .. } = &self.phase else {
            return None;
        };
        let mut unexpected = tree
            .captured_targets()
            .into_iter()
            .filter(|(identity, _durable)| !kill.closure().contains(identity))
            .peekable();
        unexpected.peek()?;
        Some(unexpected.any(|(identity, _durable)| {
            kill.closure()
                .iter()
                .any(|killed| killed.pid() == identity.pid())
        }))
    }

    fn observe_exact_generations(
        &mut self,
        tree: &CapturedProcessTree,
    ) -> anyhow::Result<Option<PostKillObservation>> {
        let PostKillPhase::Exact { kill, .. } = &self.phase else {
            return Ok(Some(PostKillObservation::Ordinary));
        };
        if kill.version() != &tree.closure_version {
            self.revoke_retained_exclusion();
            return Ok(Some(PostKillObservation::Refreeze));
        }
        let current = kill
            .closure()
            .iter()
            .map(crate::process::LiveProcessIdentity::try_matches)
            .collect::<anyhow::Result<Vec<_>>>()?
            .into_iter()
            .any(|matches| matches);
        let PostKillPhase::Exact {
            remaining_polls,
            yielded_after_exit,
            ..
        } = &mut self.phase
        else {
            unreachable!();
        };
        if current {
            if *remaining_polls == usize::default() {
                return Ok(Some(PostKillObservation::Refreeze));
            }
            *remaining_polls -= 1;
            return Ok(Some(PostKillObservation::Pending));
        }
        if !*yielded_after_exit {
            *yielded_after_exit = true;
            return Ok(Some(PostKillObservation::Pending));
        }
        Ok(None)
    }

    fn revoke_retained_exclusion(&mut self) {
        if let PostKillPhase::Exact {
            retained_exclusion, ..
        } = &mut self.phase
        {
            *retained_exclusion = RetainedExclusion::Revoked;
        }
    }

    fn observe_retained_exclusion(&mut self, tree: &mut CapturedProcessTree) -> RetainedExclusion {
        let PostKillPhase::Exact {
            retained_exclusion, ..
        } = &mut self.phase
        else {
            return RetainedExclusion::Revoked;
        };
        *retained_exclusion = retained_exclusion.observe(tree);
        *retained_exclusion
    }
}

#[cfg(target_os = "macos")]
fn observe_rebuilt_exclusion(tree: &mut CapturedProcessTree) -> PostKillObservation {
    if tree.pty_scan.proves_root_exit()
        && tree.discovery_epoch.coverage_is_complete()
        && tree.lineage_settled
        && tree.events.is_gated()
        && tree.events.post_inventory_quiescent()
    {
        if tree.events.has_unresolved_fork() {
            tree.events.acknowledge_root_exit_discovery_epoch();
            tree.begin_discovery_cleanup_internal();
            return PostKillObservation::Pending;
        }
        if !tree.lineage_integrity_lost && !tree.frozen_fork_reconciliation_required {
            return PostKillObservation::ExactEmpty;
        }
    }
    if tree.empty_output_fence_can_reconcile_uncertainty() {
        PostKillObservation::Refreeze
    } else {
        PostKillObservation::Pending
    }
}

#[cfg(not(target_os = "macos"))]
fn observe_rebuilt_exclusion(_tree: &mut CapturedProcessTree) -> PostKillObservation {
    PostKillObservation::Ordinary
}

#[cfg(all(test, target_os = "macos", feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/captured/tree/post/captured_tree_post_kill_eof_tests.rs"]
mod eof_tests;
#[cfg(all(test, target_os = "macos", feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/captured/tree/post/captured_tree_post_kill_retained_tests.rs"]
mod retained_tests;
#[cfg(all(test, target_os = "macos", feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/captured/tree/post/captured_tree_post_kill_revocation_tests.rs"]
mod revocation_tests;
#[cfg(all(test, target_os = "macos", feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/captured/tree/post/captured_tree_post_kill_tests.rs"]
mod tests;
