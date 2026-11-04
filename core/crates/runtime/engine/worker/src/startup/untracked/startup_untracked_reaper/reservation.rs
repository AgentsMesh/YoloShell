use super::super::startup_cleanup_scope::StartupCleanupScope;
use super::StartupUntrackedCleanup;
use anyhow::{Context, Result};
use std::sync::{Arc, Condvar, Mutex};

pub(crate) const REAPER_THREAD_NAME: &str = "yolo-startup-untracked-reaper";
pub(crate) const REAPER_SPAWN_CONTEXT: &str = "reserve untracked startup child reaper";
#[cfg(test)]
pub(crate) const INJECTED_RESERVATION_FAILURE: &str =
    "injected untracked reaper reservation failure";

#[cfg(test)]
thread_local! {
    static FAIL_NEXT_RESERVATION: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub(crate) struct StartupUntrackedReaperReservation {
    pub(crate) shared: Arc<SharedReservation>,
}

pub(crate) struct SharedReservation {
    pub(crate) state: Mutex<ReservationState>,
    pub(crate) ready: Condvar,
    #[cfg(test)]
    pub(crate) thread_exit: Option<Arc<std::sync::atomic::AtomicBool>>,
    #[cfg(test)]
    pub(crate) panic_on_start: bool,
}

#[derive(Default)]
pub(crate) struct ReservationState {
    pub(crate) closed: bool,
    pub(crate) cleanup: Option<Box<dyn FnMut() + Send>>,
}

impl StartupUntrackedReaperReservation {
    pub(crate) fn reserve(cleanup_scope: &mut StartupCleanupScope) -> Result<Self> {
        let (reservation, reaper) = Self::spawn_reserved(SharedReservation::new())?;
        cleanup_scope.register(reaper);
        Ok(reservation)
    }

    #[cfg(test)]
    pub(crate) fn reserve_unscoped() -> Result<Self> {
        let (reservation, reaper) = Self::spawn_reserved(SharedReservation::new())?;
        drop(reaper);
        Ok(reservation)
    }

    #[cfg(test)]
    pub(crate) fn fail_next_reservation() {
        FAIL_NEXT_RESERVATION.with(|failure| {
            assert!(!failure.replace(true));
        });
    }

    #[cfg(test)]
    pub(crate) fn reserve_observed(
        thread_exit: Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<Self> {
        let (reservation, reaper) = Self::spawn_reserved(SharedReservation::observed(thread_exit))?;
        drop(reaper);
        Ok(reservation)
    }

    #[cfg(test)]
    pub(crate) fn reserve_panicking(cleanup_scope: &mut StartupCleanupScope) -> Result<Self> {
        let (reservation, reaper) = Self::spawn_reserved(SharedReservation::panicking())?;
        cleanup_scope.register(reaper);
        Ok(reservation)
    }

    pub(crate) fn spawn_reserved(
        shared: Arc<SharedReservation>,
    ) -> Result<(Self, std::thread::JoinHandle<()>)> {
        check_injected_failure()?;
        let thread_shared = Arc::clone(&shared);
        let reaper = std::thread::Builder::new()
            .name(REAPER_THREAD_NAME.to_owned())
            .spawn(move || super::reservation_runtime::run_reserved(thread_shared))
            .context(REAPER_SPAWN_CONTEXT)?;
        Ok((Self { shared }, reaper))
    }

    pub(crate) fn handoff(self, cleanup: StartupUntrackedCleanup) {
        let mut cleanup = cleanup;
        self.handoff_task(move || cleanup.reap());
    }

    pub(crate) fn handoff_task(self, cleanup: impl FnMut() + Send + 'static) {
        let mut state = lock_state(&self.shared);
        debug_assert!(!state.closed);
        debug_assert!(state.cleanup.is_none());
        state.cleanup = Some(Box::new(cleanup));
        state.closed = true;
        drop(state);
        self.shared.ready.notify_one();
    }
}

impl Drop for StartupUntrackedReaperReservation {
    fn drop(&mut self) {
        let mut state = lock_state(&self.shared);
        state.closed = true;
        drop(state);
        self.shared.ready.notify_one();
    }
}

impl SharedReservation {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(ReservationState::default()),
            ready: Condvar::new(),
            #[cfg(test)]
            thread_exit: None,
            #[cfg(test)]
            panic_on_start: false,
        })
    }

    #[cfg(test)]
    pub(crate) fn observed(thread_exit: Arc<std::sync::atomic::AtomicBool>) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(ReservationState::default()),
            ready: Condvar::new(),
            thread_exit: Some(thread_exit),
            panic_on_start: false,
        })
    }

    #[cfg(test)]
    pub(crate) fn panicking() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(ReservationState::default()),
            ready: Condvar::new(),
            thread_exit: None,
            panic_on_start: true,
        })
    }
}

pub(crate) fn lock_state(
    shared: &SharedReservation,
) -> std::sync::MutexGuard<'_, ReservationState> {
    shared
        .state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(crate) fn check_injected_failure() -> Result<()> {
    #[cfg(test)]
    if FAIL_NEXT_RESERVATION.with(|failure| failure.replace(false)) {
        anyhow::bail!(INJECTED_RESERVATION_FAILURE);
    }
    Ok(())
}
