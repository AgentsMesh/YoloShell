use super::*;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../test_support/integration/process_file_lock_turn_test_support.rs"]
mod test_support;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../test_support/unit/process_file_lock_turn_test_support.rs"]
mod unit_test_support;
#[path = "process_file_lock_turn_waiter.rs"]
mod waiter;
#[cfg(all(test, feature = "yolo-integration-tests"))]
use test_support::run_registry_admission_hook;
#[cfg(all(test, feature = "yolo-integration-tests"))]
pub(super) use test_support::{
    arm_front_renewal_overflow_for_test, expire_front_waiter_for_test,
    insert_unowned_waiter_for_test, notify_waiters_for_test,
};

const TURN_STATE_MISSING: &str = "process lock turn state disappeared";

pub(super) struct TurnState {
    owner: bool,
    signal: Arc<wait_signal::ProcessWaitSignal>,
    waiters: VecDeque<waiter::TurnWaiter>,
}

#[derive(Debug)]
pub(crate) struct ProcessLockTurn {
    key: ObjectIdentity,
    process_id: u32,
}

impl ProcessLockTurn {
    pub(crate) fn acquire(key: ObjectIdentity, idle_timeout: Duration) -> Result<Option<Self>> {
        run_registry_admission_hook();
        let registry = current_registry();
        let waiter = waiter::TurnWaiter::idle(idle_timeout)?;
        Self::acquire_waiter(key, waiter, registry)
    }

    pub(crate) fn acquire_until(key: ObjectIdentity, deadline: Instant) -> Result<Option<Self>> {
        let registry = current_registry();
        Self::acquire_waiter(key, waiter::TurnWaiter::absolute(deadline), registry)
    }

    fn acquire_waiter(
        key: ObjectIdentity,
        waiter: waiter::TurnWaiter,
        mut registry: MutexGuard<'static, Registry>,
    ) -> Result<Option<Self>> {
        let allows_initial_grant = waiter.allows_initial_grant();
        let identity = waiter.identity();
        registry
            .turns
            .entry(key)
            .or_default()
            .waiters
            .push_back(waiter);
        if allows_initial_grant && waiter::can_grant(&registry, key, &identity) {
            waiter::grant(&mut registry, key);
            return Ok(Some(Self::new(key, std::process::id())));
        }
        loop {
            let (wake, deadline, can_grant) = waiter::observe(&registry, key, &identity);
            let remaining = match deadline.remaining(Instant::now()) {
                Ok(Some(remaining)) => remaining,
                Ok(None) => {
                    cancel_waiter(&mut registry, key, &identity);
                    return Ok(None);
                }
                Err(error) => {
                    cancel_waiter(&mut registry, key, &identity);
                    return Err(error);
                }
            };
            if can_grant {
                waiter::grant(&mut registry, key);
                return Ok(Some(Self::new(key, std::process::id())));
            }
            drop(registry);
            wake.wait_timeout(remaining);
            registry = current_registry();
        }
    }

    pub(crate) fn try_acquire(key: ObjectIdentity) -> Result<Option<Self>> {
        let mut registry = current_registry();
        let state = registry.turns.entry(key).or_default();
        if state.owner || !state.waiters.is_empty() {
            return Ok(None);
        }
        state.owner = true;
        Ok(Some(Self::new(key, std::process::id())))
    }

    pub(super) fn new(key: ObjectIdentity, process_id: u32) -> Self {
        Self { key, process_id }
    }
}

#[cfg(all(not(test), feature = "persistence-test-support"))]
impl ProcessLockTurn {
    pub(crate) fn queued_waiters_for_persistence_test(key: ObjectIdentity) -> usize {
        current_registry()
            .turns
            .get(&key)
            .map_or(usize::default(), |state| state.waiters.len())
    }
}

#[cfg(not(all(test, feature = "yolo-integration-tests")))]
fn run_registry_admission_hook() {}

impl Default for TurnState {
    fn default() -> Self {
        Self {
            owner: false,
            signal: Arc::new(wait_signal::ProcessWaitSignal::default()),
            waiters: VecDeque::new(),
        }
    }
}

impl Drop for ProcessLockTurn {
    fn drop(&mut self) {
        if let Some(signal) = release_state(self.key, self.process_id) {
            signal.notify_all();
        }
    }
}

fn release_state(
    key: ObjectIdentity,
    process_id: u32,
) -> Option<Arc<wait_signal::ProcessWaitSignal>> {
    if process_id != std::process::id() {
        return None;
    }
    let mut registry = current_registry();
    release_state_at(&mut registry, key, process_id, Instant::now())
}

fn release_state_at(
    registry: &mut Registry,
    key: ObjectIdentity,
    process_id: u32,
    occurred_at: Instant,
) -> Option<Arc<wait_signal::ProcessWaitSignal>> {
    if process_id != std::process::id() {
        return None;
    }
    let (remove, signal) = if let Some(state) = registry.turns.get_mut(&key) {
        state.owner = false;
        for waiter in &mut state.waiters {
            waiter.advance_at(occurred_at);
        }
        (state.waiters.is_empty(), Arc::clone(&state.signal))
    } else {
        return None;
    };
    if remove {
        registry.turns.remove(&key);
    }
    Some(signal)
}

pub(super) fn cancel_waiter(registry: &mut Registry, key: ObjectIdentity, waiter: &Arc<()>) {
    let (remove, signal) = if let Some(state) = registry.turns.get_mut(&key) {
        state.waiters.retain(|queued| !queued.matches(waiter));
        (
            !state.owner && state.waiters.is_empty(),
            Some(Arc::clone(&state.signal)),
        )
    } else {
        (false, None)
    };
    if remove {
        registry.turns.remove(&key);
    }
    if let Some(signal) = signal {
        signal.notify_all();
    }
}
