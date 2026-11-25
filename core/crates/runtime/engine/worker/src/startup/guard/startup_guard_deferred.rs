use super::super::pty_fault_reaper::FaultReaperReservation;
use super::super::pty_output_progress::ReaderBackedPtyOutput;
use super::super::startup_scope::{StartupScopeMonitor, TrackedTree};
use super::{
    STARTUP_AUTHORITY_UNAVAILABLE_CONTEXT, STARTUP_FAULT_REAPER_UNAVAILABLE_CONTEXT,
    STARTUP_TREE_UNAVAILABLE_CONTEXT, StartupChildGuard, StartupOutputProgress,
};
use crate::process::{CapturedProcessTree, RootWaitAuthority};
use anyhow::{Context, Result};
use std::sync::Arc;

use super::deferred_untracked::DeferredUntrackedCleanup;

pub(crate) const TRACKED_TREE_OBSERVER_DETAIL: &str =
    "deferred startup cleanup is waiting for its final process-tree observer";
pub(crate) struct DeferredTrackedCleanup {
    monitor: Option<StartupScopeMonitor>,
    retained: Option<DeferredTrackedCustody>,
    handoff: Option<super::super::pty_fault_reaper::StartupFaultHandoff>,
}

pub(crate) struct DeferredTrackedCustody {
    authority: RootWaitAuthority,
    tree: TrackedTree,
    output: StartupOutputProgress,
    fault_reaper: FaultReaperReservation,
}

impl StartupChildGuard {
    pub(crate) fn defer_drop_cleanup(&mut self) -> Result<()> {
        if self.tree.is_some() {
            self.defer_tracked_cleanup()
        } else {
            self.defer_untracked_cleanup()
        }
    }

    pub(crate) fn defer_tracked_cleanup(&mut self) -> Result<()> {
        self.authority
            .as_ref()
            .context(STARTUP_AUTHORITY_UNAVAILABLE_CONTEXT)?;
        self.fault_reaper
            .as_ref()
            .context(STARTUP_FAULT_REAPER_UNAVAILABLE_CONTEXT)?;
        self.untracked_reaper
            .as_ref()
            .context(super::untracked::UNTRACKED_REAPER_UNAVAILABLE_CONTEXT)?;
        if let Some(monitor) = self.monitor.as_ref() {
            monitor.request_stop();
        }
        let cleanup = DeferredTrackedCleanup {
            monitor: self.monitor.take(),
            retained: Some(DeferredTrackedCustody {
                authority: self
                    .authority
                    .take()
                    .context(STARTUP_AUTHORITY_UNAVAILABLE_CONTEXT)?,
                tree: self.tree.take().context(STARTUP_TREE_UNAVAILABLE_CONTEXT)?,
                output: self.output_progress.take().unwrap_or_else(|| {
                    StartupOutputProgress::ReaderBacked(
                        ReaderBackedPtyOutput::after_master_release(),
                    )
                }),
                fault_reaper: self
                    .fault_reaper
                    .take()
                    .context(STARTUP_FAULT_REAPER_UNAVAILABLE_CONTEXT)?,
            }),
            handoff: None,
        };
        let reservation = self
            .untracked_reaper
            .take()
            .context(super::untracked::UNTRACKED_REAPER_UNAVAILABLE_CONTEXT)?;
        let mut cleanup = cleanup;
        reservation.handoff_task(move || cleanup.reap());
        self.clear();
        Ok(())
    }

    pub(crate) fn defer_untracked_cleanup(&mut self) -> Result<()> {
        self.untracked_reaper
            .as_ref()
            .context(super::untracked::UNTRACKED_REAPER_UNAVAILABLE_CONTEXT)?;
        self.child
            .as_ref()
            .context(super::STARTUP_CHILD_UNAVAILABLE_CONTEXT)?;
        self.fault_reaper
            .as_ref()
            .context(STARTUP_FAULT_REAPER_UNAVAILABLE_CONTEXT)?;
        let identity = self.take_untracked_identity();
        let cleanup = DeferredUntrackedCleanup::new(
            self.child
                .take()
                .context(super::STARTUP_CHILD_UNAVAILABLE_CONTEXT)?,
            identity,
            self.output_progress.take().unwrap_or_else(|| {
                StartupOutputProgress::ReaderBacked(ReaderBackedPtyOutput::after_master_release())
            }),
            self.unreleased_gate.take(),
            self.untracked_gone_committed,
            self.fault_reaper
                .take()
                .context(STARTUP_FAULT_REAPER_UNAVAILABLE_CONTEXT)?,
        );
        let reservation = self
            .untracked_reaper
            .take()
            .context(super::untracked::UNTRACKED_REAPER_UNAVAILABLE_CONTEXT)?;
        let mut cleanup = cleanup;
        reservation.handoff_task(move || cleanup.reap());
        self.clear();
        Ok(())
    }
}

impl DeferredTrackedCleanup {
    pub(crate) fn reap(&mut self) {
        if self.handoff.is_none() {
            let failure = self
                .monitor
                .as_mut()
                .and_then(|monitor| monitor.stop().err());
            self.monitor = None;
            let retained = self
                .retained
                .take()
                .expect(STARTUP_FAULT_REAPER_UNAVAILABLE_CONTEXT);
            self.handoff = Some(retained.prepare_handoff(failure));
        }
        let handoff = self
            .handoff
            .as_mut()
            .expect(STARTUP_FAULT_REAPER_UNAVAILABLE_CONTEXT);
        handoff.accept();
        handoff.wait_for_publication();
        self.handoff = None;
    }
}

impl DeferredTrackedCustody {
    pub(crate) fn prepare_handoff(
        self,
        failure: Option<anyhow::Error>,
    ) -> super::super::pty_fault_reaper::StartupFaultHandoff {
        self.fault_reaper.prepare_startup_handoff(
            self.authority,
            wait_for_exclusive_tree(self.tree),
            self.output,
            failure,
        )
    }
}

pub(crate) fn wait_for_exclusive_tree(mut tree: TrackedTree) -> CapturedProcessTree {
    loop {
        match Arc::try_unwrap(tree) {
            Ok(tree) => {
                return tree
                    .into_inner()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
            Err(retained) => tree = retained,
        }
        super::super::terminal_reporting::debug_message(TRACKED_TREE_OBSERVER_DETAIL);
        std::thread::sleep(super::super::constants::STARTUP_CHILD_EXIT_POLL_INTERVAL);
    }
}
