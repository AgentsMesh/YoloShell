use super::super::pty_fault_reaper::{settle_inline, settle_inline_after_master_release};
use super::{STARTUP_CHILD_UNAVAILABLE_CONTEXT, StartupChildGuard, StartupOutputProgress};
use crate::process::{CapturedProcessTree, DirectChildIdentity, RootWaitAuthority};
use anyhow::{Context, Result};

pub(crate) struct PromotedUntrackedChild {
    pub(crate) authority: RootWaitAuthority,
    pub(crate) tree: CapturedProcessTree,
}

impl StartupChildGuard {
    pub(crate) fn promote_untracked_child(&mut self) -> Result<PromotedUntrackedChild> {
        let child = self
            .child
            .as_ref()
            .context(STARTUP_CHILD_UNAVAILABLE_CONTEXT)?;
        let direct = DirectChildIdentity::capture_candidate(child)?;
        direct.prove_current(child)?;
        let tree = CapturedProcessTree::capture(direct.root())?;
        let binding = RootWaitAuthority::prove_owned_child(child, &tree)?;
        let child = self
            .child
            .take()
            .context(STARTUP_CHILD_UNAVAILABLE_CONTEXT)?;
        Ok(PromotedUntrackedChild {
            authority: binding.bind(child),
            tree,
        })
    }
}

impl PromotedUntrackedChild {
    pub(crate) fn settle(
        self,
        guard: &mut StartupChildGuard,
        output: StartupOutputProgress,
        failure: Option<anyhow::Error>,
    ) -> Result<()> {
        let state_observer = guard
            .fault_reaper
            .as_ref()
            .context(super::STARTUP_FAULT_REAPER_UNAVAILABLE_CONTEXT)?
            .state_observer();
        let outcome = match output {
            StartupOutputProgress::Exclusive(output) => settle_inline_after_master_release(
                self.authority,
                self.tree,
                output,
                failure,
                state_observer,
            ),
            StartupOutputProgress::ReaderBacked(output) => {
                settle_inline(self.authority, self.tree, output, failure, state_observer)
            }
        };
        guard.clear();
        outcome.fault.map_or(Ok(()), Err)
    }
}
