use anyhow::{Result, anyhow};
use std::process::{Child, Command};
use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::Instant;

static SPAWN_GATE: SpawnGate = SpawnGate {
    state: Mutex::new(SpawnGateState {
        active: false,
        waiting_spawners: 0,
    }),
    ready: Condvar::new(),
};

struct SpawnGate {
    state: Mutex<SpawnGateState>,
    ready: Condvar,
}

struct SpawnGateState {
    active: bool,
    waiting_spawners: usize,
}

pub struct WaitableChildSpawnGuard {
    active: bool,
}

pub struct DescriptorScanGuard {
    _spawn_gate: WaitableChildSpawnGuard,
}

pub struct WaitableChildSpawnBoundary<'guard> {
    _guard: &'guard WaitableChildSpawnGuard,
}

impl WaitableChildSpawnGuard {
    pub fn boundary(&self) -> WaitableChildSpawnBoundary<'_> {
        debug_assert!(self.active);
        WaitableChildSpawnBoundary { _guard: self }
    }

    #[cfg(any(test, feature = "yolo-test-support"))]
    pub fn waiting_spawners_for_test(&self) -> usize {
        debug_assert!(self.active);
        lock_spawn_gate().waiting_spawners
    }
}

impl<'guard> WaitableChildSpawnBoundary<'guard> {
    pub fn spawn_command(&self, command: &mut Command) -> std::io::Result<Child> {
        command.spawn()
    }
}

pub fn waitable_child_spawn_guard() -> Result<WaitableChildSpawnGuard> {
    acquire_spawn_gate(None)
}

pub fn waitable_child_spawn_guard_before(
    deadline: Instant,
) -> Result<WaitableChildSpawnGuard> {
    acquire_spawn_gate(Some(deadline))
}

pub fn try_waitable_child_spawn_guard() -> Result<Option<WaitableChildSpawnGuard>> {
    #[cfg(any(test, feature = "yolo-test-support"))]
    crate::test_hook::reject_gate_entry();
    let mut state = lock_spawn_gate();
    if state.active || state.waiting_spawners != usize::default() {
        return Ok(None);
    }
    state.active = true;
    drop(state);
    let guard = WaitableChildSpawnGuard { active: true };
    crate::normalize_sigchld_waitability()?;
    Ok(Some(guard))
}

pub fn try_descriptor_scan_guard() -> Option<DescriptorScanGuard> {
    let mut state = lock_spawn_gate();
    if state.active || state.waiting_spawners != usize::default() {
        return None;
    }
    #[cfg(any(test, feature = "yolo-test-support"))]
    crate::test_hook::reject_gate_entry();
    state.active = true;
    Some(DescriptorScanGuard {
        _spawn_gate: WaitableChildSpawnGuard { active: true },
    })
}

fn acquire_spawn_gate(deadline: Option<Instant>) -> Result<WaitableChildSpawnGuard> {
    #[cfg(any(test, feature = "yolo-test-support"))]
    crate::test_hook::reject_gate_entry();
    let mut state = lock_spawn_gate();
    state.waiting_spawners += 1;
    let (mut state, result) = wait_until_available(state, deadline);
    state.waiting_spawners -= 1;
    if let Err(error) = result {
        SPAWN_GATE.ready.notify_one();
        return Err(error);
    }
    state.active = true;
    drop(state);
    let guard = WaitableChildSpawnGuard { active: true };
    crate::normalize_sigchld_waitability()?;
    Ok(guard)
}

fn wait_until_available(
    mut state: MutexGuard<'static, SpawnGateState>,
    deadline: Option<Instant>,
) -> (MutexGuard<'static, SpawnGateState>, Result<()>) {
    while state.active {
        state = match deadline {
            Some(deadline) => {
                let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                    return (state, deadline_error());
                };
                if remaining.is_zero() {
                    return (state, deadline_error());
                }
                SPAWN_GATE
                    .ready
                    .wait_timeout(state, remaining)
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .0
            }
            None => SPAWN_GATE
                .ready
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        };
    }
    let result = deadline.map_or(Ok(()), crate::require_budget);
    (state, result)
}

fn deadline_error() -> Result<()> {
    Err(anyhow!(crate::PRIVATE_INPUT_DEADLINE_CONTEXT))
}

impl Drop for WaitableChildSpawnGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        lock_spawn_gate().active = false;
        self.active = false;
        SPAWN_GATE.ready.notify_one();
    }
}

fn lock_spawn_gate() -> MutexGuard<'static, SpawnGateState> {
    SPAWN_GATE
        .state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(any(test, feature = "yolo-test-support"))]
pub struct ReservedTestSpawnWaiter;

#[cfg(any(test, feature = "yolo-test-support"))]
pub fn reserve_spawn_waiter_for_test() -> ReservedTestSpawnWaiter {
    lock_spawn_gate().waiting_spawners += 1;
    ReservedTestSpawnWaiter
}

#[cfg(any(test, feature = "yolo-test-support"))]
impl Drop for ReservedTestSpawnWaiter {
    fn drop(&mut self) {
        lock_spawn_gate().waiting_spawners -= 1;
        SPAWN_GATE.ready.notify_one();
    }
}
