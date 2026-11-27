use super::FaultReaperReservation;
use super::fallback::FaultReaperFallback;
use super::fault_reap_construction::vacant_fault_reap;
use super::ownership_handoff::OwnershipFaultHandoff;
use super::reservation_claim::FaultReapClaim;
use crate::worker::supervision;
use std::panic::{AssertUnwindSafe, catch_unwind};

pub(crate) const CLAIM_INSTALL_FAILURE_DETAIL: &str =
    "vacant PTY ownership claim rejected complete custody";
pub(crate) const BUILD_PANIC_DETAIL: &str = "PTY ownership handoff post-install hook panicked";
pub(crate) const DISPATCH_PANIC_DETAIL: &str = "PTY ownership handoff dispatch panicked";

#[cfg(test)]
pub(crate) const INJECTED_BUILD_PANIC: &str = "injected PTY ownership post-install panic";
#[cfg(test)]
pub(crate) const INJECTED_DISPATCH_PANIC: &str = "injected PTY ownership handoff dispatch panic";

pub(crate) struct OwnershipHandoffTransaction {
    pub(crate) claim: FaultReapClaim,
    pub(crate) fallback: FaultReaperFallback,
    pub(crate) installed: bool,
    #[cfg(test)]
    pub(crate) post_install_panics: bool,
}

impl OwnershipHandoffTransaction {
    pub(crate) fn prepare(reservation: &mut FaultReaperReservation) -> Self {
        let schedule = super::schedule::FaultReapSchedule::production();
        let control_plane = vacant_fault_reap(reservation.state_observer(), schedule);
        let transaction = Self {
            claim: FaultReapClaim::vacant(control_plane),
            fallback: reservation.fallback.clone(),
            installed: false,
            #[cfg(test)]
            post_install_panics: reservation.take_handoff_build_panic_for_test(),
        };
        transaction.anchor(reservation);
        transaction
    }

    pub(crate) fn install(mut self, handoff: &mut Option<OwnershipFaultHandoff>) {
        handoff
            .as_mut()
            .expect(CLAIM_INSTALL_FAILURE_DETAIL)
            .prepare();
        assert!(
            self.claim.install(handoff),
            "{CLAIM_INSTALL_FAILURE_DETAIL}"
        );
        self.installed = true;
        self.run_post_install_hook();
    }

    pub(crate) fn anchor(&self, reservation: &mut FaultReaperReservation) {
        let claim = self.claim.clone();
        let result = catch_unwind(AssertUnwindSafe(|| {
            #[cfg(test)]
            if reservation.take_handoff_dispatch_panic_for_test() {
                panic!("{INJECTED_DISPATCH_PANIC}");
            }
            reservation.sender.send(claim)
        }));
        match result {
            Ok(Ok(())) => {}
            Ok(Err(failure)) => {
                self.fallback.handoff(failure.0);
                crate::worker::terminal_reporting::warn_message(
                    super::constants::PRIMARY_HANDOFF_FAILURE_DETAIL,
                );
            }
            Err(payload) => {
                report_panic(payload, DISPATCH_PANIC_DETAIL);
                self.fallback.handoff(self.claim.clone());
            }
        }
    }

    pub(crate) fn run_post_install_hook(&mut self) {
        #[cfg(test)]
        if std::mem::take(&mut self.post_install_panics) {
            let result = catch_unwind(AssertUnwindSafe(|| -> () {
                panic!("{INJECTED_BUILD_PANIC}")
            }));
            if let Err(payload) = result {
                report_panic(payload, BUILD_PANIC_DETAIL);
            }
        }
    }
}

impl Drop for OwnershipHandoffTransaction {
    fn drop(&mut self) {
        if !self.installed {
            self.claim.cancel();
        }
    }
}

pub(crate) fn report_panic(payload: Box<dyn std::any::Any + Send>, context: &'static str) {
    let detail = supervision::panic_detail(payload);
    crate::worker::terminal_reporting::warn_detail(&detail, context);
}
