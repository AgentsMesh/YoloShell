use super::{DetachedChildReservation, TerminationPolicy, prepare_detached_child_handoff};
use anyhow::Result;
use std::process::Child;

const CUSTODY_CAPTURE_FAILED: &str =
    "reserved detached child settlement could not capture exact identity; retaining wait custody";

pub struct ReservedDetachedChildSettlement {
    reservation: DetachedChildReservation,
}

pub fn reserve_detached_child_settlement() -> Result<ReservedDetachedChildSettlement> {
    prepare_detached_child_handoff().map(ReservedDetachedChildSettlement::new)
}

impl ReservedDetachedChildSettlement {
    pub fn retain(self, child: Child) {
        self.settle(child, TerminationPolicy::NaturalExit);
    }

    pub fn terminate(self, child: Child) {
        self.settle(child, TerminationPolicy::Terminate);
    }

    fn settle(self, child: Child, policy: TerminationPolicy) {
        let custody = match crate::process::DetachedChildCustody::capture(child) {
            Ok(custody) => custody,
            Err(error) => {
                let (source, custody) = error.into_source_and_custody();
                super::containment::report_error(source, CUSTODY_CAPTURE_FAILED);
                custody
            }
        };
        self.reservation.retain(custody, policy);
    }

    pub(in crate::process) fn terminate_custody(
        self,
        custody: crate::process::DetachedChildCustody,
    ) {
        self.reservation
            .retain(custody, TerminationPolicy::Terminate);
    }

    pub(in crate::process) fn retain_custody(self, custody: crate::process::DetachedChildCustody) {
        self.reservation
            .retain(custody, TerminationPolicy::NaturalExit);
    }

    pub(super) fn new(reservation: DetachedChildReservation) -> Self {
        Self { reservation }
    }
}
