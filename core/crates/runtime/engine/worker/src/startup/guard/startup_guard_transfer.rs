use super::super::pty_ownership::PtyOwnership;
use super::super::startup_scope::new_tracked_tree;
#[cfg(test)]
use super::STARTUP_READER_NOT_INSTALLED_CONTEXT;
use super::{
    ACTIVE_OBSERVER_DETAIL, CHILD_IDENTITY_MISMATCH_DETAIL, STARTUP_AUTHORITY_UNAVAILABLE_CONTEXT,
    STARTUP_TREE_UNAVAILABLE_CONTEXT, StartupChildGuard, StartupOutputProgress,
};
use crate::process::{CapturedProcessTree, RootWaitAuthority};
use anyhow::{Context, Result, bail, ensure};
use std::sync::Arc;

pub(crate) const PREPARED_OWNERSHIP_INVARIANT: &str =
    "prepared PTY ownership transfer remains valid until consumption";

impl StartupChildGuard {
    pub(crate) fn destructive_session_capability(
        &self,
    ) -> std::result::Result<(), crate::process::DestructiveSessionUnavailable> {
        let tree = self
            .tree
            .as_ref()
            .unwrap_or_else(|| unreachable!("{PREPARED_OWNERSHIP_INVARIANT}"));
        tree.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .require_destructive_session()
    }

    #[cfg(test)]
    pub(crate) fn take_ownership(&mut self) -> Result<PtyOwnership> {
        self.prepare_ownership_transfer()?;
        match self.output_progress.as_ref() {
            Some(StartupOutputProgress::ReaderBacked(_)) => {}
            Some(StartupOutputProgress::Exclusive(_)) => {
                bail!(STARTUP_READER_NOT_INSTALLED_CONTEXT)
            }
            None => bail!(super::STARTUP_OUTPUT_PROGRESS_UNAVAILABLE_CONTEXT),
        }
        Ok(self.take_prepared_ownership())
    }

    pub(crate) fn prepare_ownership_transfer(&mut self) -> Result<()> {
        self.stop_monitor()?;
        ensure!(
            self.process_id == self.identity.map(|identity| identity.pid),
            CHILD_IDENTITY_MISMATCH_DETAIL
        );
        self.authority
            .as_ref()
            .context(STARTUP_AUTHORITY_UNAVAILABLE_CONTEXT)?;
        self.fault_reaper
            .as_ref()
            .context(super::STARTUP_FAULT_REAPER_UNAVAILABLE_CONTEXT)?;
        let tree = self
            .tree
            .as_ref()
            .context(STARTUP_TREE_UNAVAILABLE_CONTEXT)?;
        ensure!(Arc::strong_count(tree) == 1, ACTIVE_OBSERVER_DETAIL);
        Ok(())
    }

    pub(crate) fn take_prepared_ownership(&mut self) -> PtyOwnership {
        let (authority, tree) = self
            .take_stopped_authority()
            .unwrap_or_else(|_| unreachable!("{PREPARED_OWNERSHIP_INVARIANT}"));
        let Some(StartupOutputProgress::ReaderBacked(output_progress)) =
            self.output_progress.take()
        else {
            unreachable!("{PREPARED_OWNERSHIP_INVARIANT}")
        };
        let fault_reaper = self
            .fault_reaper
            .take()
            .unwrap_or_else(|| unreachable!("{PREPARED_OWNERSHIP_INVARIANT}"));
        let ownership =
            PtyOwnership::from_authority(authority, tree, output_progress, fault_reaper);
        self.disarm_after_transfer();
        ownership
    }

    pub(crate) fn take_stopped_authority(
        &mut self,
    ) -> Result<(RootWaitAuthority, CapturedProcessTree)> {
        ensure!(
            self.process_id == self.identity.map(|identity| identity.pid),
            CHILD_IDENTITY_MISMATCH_DETAIL
        );
        self.authority
            .as_ref()
            .context(STARTUP_AUTHORITY_UNAVAILABLE_CONTEXT)?;
        let tree = self.take_stopped_tree()?;
        let Some(authority) = self.authority.take() else {
            self.tree = Some(new_tracked_tree(tree));
            bail!(STARTUP_AUTHORITY_UNAVAILABLE_CONTEXT);
        };
        Ok((authority, tree))
    }

    pub(crate) fn take_stopped_tree(&mut self) -> Result<CapturedProcessTree> {
        let tracked = self.tree.take().context(STARTUP_TREE_UNAVAILABLE_CONTEXT)?;
        match Arc::try_unwrap(tracked) {
            Ok(tree) => Ok(tree
                .into_inner()
                .unwrap_or_else(std::sync::PoisonError::into_inner)),
            Err(tracked) => {
                self.tree = Some(tracked);
                bail!(ACTIVE_OBSERVER_DETAIL)
            }
        }
    }
}
