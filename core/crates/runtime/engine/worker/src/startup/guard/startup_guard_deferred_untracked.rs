use super::super::pty_fault_reaper::FaultReaperReservation;
use super::super::startup_untracked_reaper::StartupUntrackedCleanup;
use super::StartupOutputProgress;
use crate::process::{CapturedProcessTree, RootWaitAuthority};

pub(crate) const UNTRACKED_BIND_FAILURE_DETAIL: &str =
    "deferred startup cleanup could not bind exact child custody";
pub(crate) const UNTRACKED_BIND_PANIC_DETAIL: &str =
    "deferred startup cleanup identity binding panicked";

pub(crate) struct DeferredUntrackedCleanup {
    pub(crate) retained: Option<DeferredUntrackedCustody>,
    pub(crate) handoff: Option<super::super::pty_fault_reaper::StartupFaultHandoff>,
    pub(crate) fallback: Option<StartupUntrackedCleanup>,
}

pub(crate) struct DeferredUntrackedCustody {
    child: super::PtyChild,
    identity: super::super::startup_untracked_reaper::StartupUntrackedIdentity,
    output: StartupOutputProgress,
    gate: Option<super::super::UnreleasedBootstrapGate>,
    producers_gone: bool,
    fault_reaper: FaultReaperReservation,
}

impl DeferredUntrackedCleanup {
    pub(crate) fn new(
        child: super::PtyChild,
        identity: super::super::startup_untracked_reaper::StartupUntrackedIdentity,
        output: StartupOutputProgress,
        gate: Option<super::super::UnreleasedBootstrapGate>,
        producers_gone: bool,
        fault_reaper: FaultReaperReservation,
    ) -> Self {
        Self {
            retained: Some(DeferredUntrackedCustody {
                child,
                identity,
                output,
                gate,
                producers_gone,
                fault_reaper,
            }),
            handoff: None,
            fallback: None,
        }
    }

    pub(crate) fn reap(&mut self) {
        if let Some(fallback) = self.fallback.as_mut() {
            fallback.reap();
            return;
        }
        if self.handoff.is_some() {
            self.finish_handoff();
            return;
        }
        let retained = self
            .retained
            .as_ref()
            .expect(super::STARTUP_CHILD_UNAVAILABLE_CONTEXT);
        let binding = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let child = &retained.child;
            let direct = crate::process::DirectChildIdentity::capture_candidate(child)?;
            direct.prove_current(child)?;
            let tree = CapturedProcessTree::capture(direct.root())?;
            let binding = RootWaitAuthority::prove_owned_child(child, &tree)?;
            Ok::<_, anyhow::Error>((binding, tree))
        }));
        match binding {
            Ok(Ok((binding, tree))) => self.handoff_tracked(binding, tree),
            Ok(Err(error)) => self.install_fallback(error),
            Err(payload) => {
                let detail = super::super::supervision::panic_detail(payload);
                self.install_fallback(anyhow::anyhow!(detail).context(UNTRACKED_BIND_PANIC_DETAIL));
            }
        }
        if let Some(fallback) = self.fallback.as_mut() {
            fallback.reap();
        } else if self.handoff.is_some() {
            self.finish_handoff();
        }
    }

    pub(crate) fn handoff_tracked(
        &mut self,
        binding: crate::process::RootWaitBinding,
        tree: CapturedProcessTree,
    ) {
        let retained = self
            .retained
            .take()
            .expect(super::STARTUP_CHILD_UNAVAILABLE_CONTEXT);
        self.handoff = Some(retained.fault_reaper.prepare_startup_handoff(
            binding.bind(retained.child),
            tree,
            retained.output,
            None,
        ));
    }

    pub(crate) fn finish_handoff(&mut self) {
        let handoff = self
            .handoff
            .as_mut()
            .expect(super::STARTUP_CHILD_UNAVAILABLE_CONTEXT);
        handoff.accept();
        handoff.wait_for_publication();
        self.handoff = None;
    }

    pub(crate) fn install_fallback(&mut self, error: anyhow::Error) {
        super::super::terminal_reporting::warn_error(error, UNTRACKED_BIND_FAILURE_DETAIL);
        let retained = self
            .retained
            .take()
            .expect(super::STARTUP_CHILD_UNAVAILABLE_CONTEXT);
        drop(retained.fault_reaper);
        self.fallback = Some(StartupUntrackedCleanup::with_deadline(
            retained.child,
            retained.identity,
            retained.output,
            retained.gate,
            retained.producers_gone,
            false,
            super::super::pty_missing_eof::MissingEofDeadline::production(),
        ));
    }
}
