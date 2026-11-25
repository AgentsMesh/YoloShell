use super::reservation_claim::FaultReapClaim;
use super::{FaultReapCompletion, FaultReaperReservation};

#[cfg(test)]
pub(crate) const INJECTED_STARTUP_ACCEPT_PANIC: &str =
    "injected startup fault handoff accept panic";

pub(crate) struct StartupFaultHandoff {
    pub(crate) reservation: FaultReaperReservation,
    pub(crate) claim: FaultReapClaim,
    pub(crate) settlement: FaultReapCompletion,
    pub(crate) committed: bool,
}

impl StartupFaultHandoff {
    pub(crate) fn new(
        reservation: FaultReaperReservation,
        claim: FaultReapClaim,
        settlement: FaultReapCompletion,
    ) -> Self {
        Self {
            reservation,
            claim,
            settlement,
            committed: false,
        }
    }

    pub(crate) fn accept(&mut self) {
        #[cfg(test)]
        if std::mem::take(&mut self.reservation.startup_accept_panics) {
            panic!("{INJECTED_STARTUP_ACCEPT_PANIC}");
        }
        self.commit();
    }

    pub(crate) fn wait_for_publication(&self) {
        self.settlement.wait_for_publication();
    }

    pub(crate) fn commit(&mut self) {
        if self.committed {
            return;
        }
        if let Err(failure) = self.reservation.sender.send(self.claim.clone()) {
            self.reservation.fallback.handoff(failure.0);
            super::super::terminal_reporting::warn_message(
                super::constants::PRIMARY_HANDOFF_FAILURE_DETAIL,
            );
        }
        self.committed = true;
    }
}

impl Drop for StartupFaultHandoff {
    fn drop(&mut self) {
        self.commit();
    }
}
