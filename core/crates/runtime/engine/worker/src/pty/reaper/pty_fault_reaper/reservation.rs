use super::fault_reap_construction::fault_reap;
use super::reservation_claim::FaultReapClaim;
use super::{
    FaultOutput, FaultReapCompletion, FaultReaperReservation, PtyForceTermination,
    PtyProcessCustody, RootWaitAuthority, StartupFaultHandoff,
};
use anyhow::Result;
use std::sync::mpsc::sync_channel;

pub(crate) const OWNERSHIP_CHANNEL_CAPACITY: usize = 1;
#[cfg(test)]
pub(crate) const FAULT_CUSTODY_PRESENT_DETAIL: &str = "fault custody is present";

impl FaultReaperReservation {
    pub(crate) fn reserve() -> Result<Self> {
        super::reservation_runtime::check_injected_failure()?;
        Self::reserve_with_primary(super::reservation_runtime::primary_should_run())
    }

    #[cfg(test)]
    pub(crate) fn reserve_primary_closed_for_test() -> Result<Self> {
        Self::reserve_with_primary(false)
    }

    pub(crate) fn reserve_with_primary(primary_running: bool) -> Result<Self> {
        let state_observer = super::FaultReapStateObserver::default();
        let emergency =
            super::emergency_escrow::EmergencyEscrowProducer::reserve(state_observer.clone())?;
        let fallback = super::fallback::FaultReaperFallback::capture()?;
        let (sender, receiver) = sync_channel::<FaultReapClaim>(OWNERSHIP_CHANNEL_CAPACITY);
        if primary_running {
            super::reservation_runtime::spawn_receiver(receiver)?;
        } else {
            drop(receiver);
        }
        Ok(Self {
            sender,
            fallback,
            emergency: Some(emergency),
            #[cfg(test)]
            startup_completion: None,
            #[cfg(test)]
            handoff_observer: None,
            state_observer,
            #[cfg(test)]
            startup_accept_panics: false,
            #[cfg(test)]
            startup_drive_panics: false,
            #[cfg(test)]
            handoff_build_panics: false,
            #[cfg(test)]
            handoff_dispatch_panics: false,
            #[cfg(test)]
            retirement: None,
        })
    }

    #[cfg(test)]
    pub(crate) fn fail_next_reservation() {
        super::reservation_runtime::fail_next_reservation();
    }

    #[cfg(test)]
    pub(crate) fn close_next_primary_for_test() {
        super::reservation_runtime::close_next_primary();
    }

    #[cfg(test)]
    pub(crate) fn panic_on_startup_accept_for_test(&mut self) {
        assert!(!self.startup_accept_panics);
        self.startup_accept_panics = true;
    }

    #[cfg(test)]
    pub(crate) fn panic_on_startup_drive_for_test(&mut self) {
        assert!(!self.startup_drive_panics);
        self.startup_drive_panics = true;
    }

    pub(crate) fn prepare_startup_handoff(
        mut self,
        authority: RootWaitAuthority,
        tree: crate::process::CapturedProcessTree,
        output: crate::worker::startup::startup_guard::StartupOutputProgress,
        failure: Option<anyhow::Error>,
    ) -> StartupFaultHandoff {
        let output = match output {
            crate::worker::startup::startup_guard::StartupOutputProgress::Exclusive(output) => {
                FaultOutput::Exclusive(output)
            }
            crate::worker::startup::startup_guard::StartupOutputProgress::ReaderBacked(output) => {
                FaultOutput::Reader(output)
            }
        };
        let completion = self.take_startup_completion();
        let settlement = completion.clone();
        let ownership = fault_reap(
            authority,
            PtyProcessCustody::tracking(tree),
            PtyForceTermination::new(),
            output,
            crate::worker::pty_missing_eof::MissingEofDeadline::production(),
            None,
            failure,
            completion,
            self.state_observer.clone(),
        );
        #[cfg(test)]
        self.observe_handoff(
            ownership
                .custody()
                .processes
                .as_ref()
                .expect(FAULT_CUSTODY_PRESENT_DETAIL),
        );
        let claim = FaultReapClaim::new(ownership);
        #[cfg(test)]
        if std::mem::take(&mut self.startup_drive_panics) {
            claim.panic_before_drive_for_test();
        }
        StartupFaultHandoff::new(self, claim, settlement)
    }

    #[cfg(test)]
    pub(crate) fn observe_startup_completion(&mut self) -> super::FaultReapReceiver {
        let (completion, receiver) = FaultReapCompletion::owner_channel();
        assert!(self.startup_completion.replace(completion).is_none());
        receiver
    }

    pub(crate) fn take_startup_completion(&mut self) -> FaultReapCompletion {
        #[cfg(test)]
        if let Some(completion) = self.startup_completion.take() {
            return completion;
        }
        FaultReapCompletion::log()
    }

    pub(crate) fn state_observer(&self) -> super::FaultReapStateObserver {
        self.state_observer.clone()
    }
}
