#[cfg(test)]
use super::FaultReaperReservation;
use anyhow::Result;

#[cfg(test)]
pub(crate) const INJECTED_RESERVATION_FAILURE: &str =
    "injected PTY emergency escrow reservation failure";

#[cfg(test)]
thread_local! {
    static FAIL_NEXT_RESERVATION: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub(crate) fn check() -> Result<()> {
    #[cfg(test)]
    if FAIL_NEXT_RESERVATION.with(|failure| failure.replace(false)) {
        anyhow::bail!(INJECTED_RESERVATION_FAILURE);
    }
    Ok(())
}

#[cfg(test)]
impl FaultReaperReservation {
    pub(crate) fn fail_next_emergency_reservation_for_test() {
        FAIL_NEXT_RESERVATION.with(|failure| assert!(!failure.replace(true)));
    }
}
