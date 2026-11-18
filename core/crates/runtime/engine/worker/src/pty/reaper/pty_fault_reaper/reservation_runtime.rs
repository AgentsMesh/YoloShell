use super::reservation_claim::FaultReapClaim;
use anyhow::{Context, Result};

pub(crate) const REAPER_SPAWN_CONTEXT: &str = "reserve PTY ownership fault reaper";
#[cfg(test)]
pub(crate) const INJECTED_RESERVATION_FAILURE: &str =
    "injected PTY fault reaper reservation failure";

#[cfg(test)]
thread_local! {
    static FAIL_NEXT_RESERVATION: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static CLOSE_NEXT_PRIMARY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub(crate) fn spawn_receiver(receiver: std::sync::mpsc::Receiver<FaultReapClaim>) -> Result<()> {
    std::thread::Builder::new()
        .name(super::constants::PTY_FAULT_REAPER_THREAD_NAME.to_owned())
        .spawn(move || {
            while let Ok(claim) = receiver.recv() {
                claim.reap();
            }
        })
        .context(REAPER_SPAWN_CONTEXT)?;
    Ok(())
}

pub(crate) fn check_injected_failure() -> Result<()> {
    #[cfg(test)]
    if FAIL_NEXT_RESERVATION.with(|failure| failure.replace(false)) {
        anyhow::bail!(INJECTED_RESERVATION_FAILURE);
    }
    Ok(())
}

pub(crate) fn primary_should_run() -> bool {
    #[cfg(test)]
    return !CLOSE_NEXT_PRIMARY.with(|closed| closed.replace(false));
    #[cfg(not(test))]
    true
}

#[cfg(test)]
pub(crate) fn fail_next_reservation() {
    FAIL_NEXT_RESERVATION.with(|failure| assert!(!failure.replace(true)));
}

#[cfg(test)]
pub(crate) fn close_next_primary() {
    CLOSE_NEXT_PRIMARY.with(|closed| assert!(!closed.replace(true)));
}
