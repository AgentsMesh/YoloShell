use super::{
    FaultReap, FaultReapCompletion, FaultReapStateObserver, FaultReaperReservation,
    OwnershipFaultHandoff,
};
use crate::worker::pty_ownership::PtyOwnershipCustody;
use anyhow::{Context, Result};
use std::any::Any;
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../../unit_tests/pty_fault_reaper/emergency_escrow_adversarial_tests.rs"]
mod adversarial_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../../../test_support/unit/pty_fault_reaper/emergency_escrow_fixture.rs"]
mod fixture;
#[path = "emergency_escrow_install.rs"]
mod install;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../../unit_tests/pty_fault_reaper/emergency_escrow_regression_tests.rs"]
mod regression_tests;
#[path = "emergency_escrow_reservation_failure.rs"]
mod reservation_failure;
#[path = "emergency_escrow_runtime.rs"]
mod runtime;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../../unit_tests/pty_fault_reaper/emergency_escrow_spawn_tests.rs"]
mod spawn_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../../../test_support/unit/pty_fault_reaper/emergency_escrow_test_support.rs"]
mod test_support;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../../unit_tests/pty_fault_reaper/emergency_escrow_tests.rs"]
mod tests;

pub(crate) const DISPATCHER_THREAD_NAME: &str = "yolo-pty-emergency-escrow-dispatcher";
pub(crate) const OWNER_THREAD_NAME: &str = "yolo-pty-emergency-escrow-owner";
pub(crate) const DISPATCHER_SPAWN_CONTEXT: &str =
    "spawn process-global PTY emergency escrow dispatcher";
pub(crate) const OWNERSHIP_DROP_DETAIL: &str = "PTY ownership dropped before explicit settlement";
pub(crate) const DROP_PANIC_DETAIL: &str = "PTY ownership drop settlement panicked";
pub(crate) const DRIVER_PANIC_DETAIL: &str = "PTY emergency escrow driver panicked";
pub(crate) const OWNER_SPAWN_FAILURE_DETAIL: &str = "spawn PTY emergency escrow owner";
pub(crate) const OWNER_SPAWN_PANIC_DETAIL: &str = "PTY emergency escrow owner spawn panicked";
#[cfg(test)]
pub(crate) const INJECTED_OWNER_SPAWN_FAILURE_DETAIL: &str =
    "injected PTY emergency escrow owner spawn failure";
#[cfg(test)]
pub(crate) const INJECTED_OWNER_SPAWN_PANIC_DETAIL: &str =
    "injected PTY emergency escrow owner spawn panic";

pub(crate) static REGISTRY: OnceLock<std::result::Result<EmergencyRegistry, String>> =
    OnceLock::new();

pub(crate) struct EmergencyEscrowProducer {
    pub(crate) slot: Option<Arc<EmergencySlot>>,
    pub(crate) control: Option<FaultReap>,
    pub(crate) completion: Option<FaultReapCompletion>,
}

pub(crate) struct EmergencyEnvelope {
    reap: FaultReap,
    panic: Option<Box<dyn Any + Send>>,
    _retired_completion: Option<FaultReapCompletion>,
    _retired_reservation: Option<FaultReaperReservation>,
    _producer_slot: Arc<EmergencySlot>,
}

pub(crate) struct EmergencySlot {
    phase: AtomicU8,
    envelope: UnsafeCell<MaybeUninit<EmergencyEnvelope>>,
    dispatcher: std::thread::Thread,
    #[cfg(test)]
    test_hooks: test_support::EmergencySlotTestHooks,
}

// SAFETY: the unique producer writes before Ready/Release. The dispatcher is the
// only consumer and reads only after a successful Ready->Driving Acquire CAS.
unsafe impl Send for EmergencySlot {}
// SAFETY: the phase protocol prevents concurrent envelope mutation or drop.
unsafe impl Sync for EmergencySlot {}

#[derive(Clone)]
pub(crate) struct EmergencyRegistry {
    slots: Arc<Mutex<Vec<Arc<EmergencySlot>>>>,
    dispatcher: std::thread::Thread,
}

#[derive(Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub(crate) enum EmergencyPhase {
    Vacant,
    Ready,
    Driving,
    Settled,
    Cancelled,
}

impl EmergencyEscrowProducer {
    pub(crate) fn reserve(observer: FaultReapStateObserver) -> Result<Self> {
        reservation_failure::check()?;
        let registry = capture_registry()?;
        let slot = Arc::new(EmergencySlot {
            phase: AtomicU8::new(EmergencyPhase::Vacant as u8),
            envelope: UnsafeCell::new(MaybeUninit::uninit()),
            dispatcher: registry.dispatcher.clone(),
            #[cfg(test)]
            test_hooks: test_support::EmergencySlotTestHooks::default(),
        });
        registry.register(Arc::clone(&slot));
        Ok(Self {
            slot: Some(slot),
            control: Some(super::fault_reap_construction::vacant_fault_reap(
                observer,
                super::schedule::FaultReapSchedule::production(),
            )),
            completion: Some(FaultReapCompletion::log()),
        })
    }
}

impl Drop for EmergencyEscrowProducer {
    fn drop(&mut self) {
        let Some(slot) = self.slot.as_ref() else {
            return;
        };
        if slot
            .phase
            .compare_exchange(
                EmergencyPhase::Vacant as u8,
                EmergencyPhase::Cancelled as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
        {
            slot.dispatcher.unpark();
        }
    }
}

impl EmergencyRegistry {
    pub(crate) fn register(&self, slot: Arc<EmergencySlot>) {
        lock(&self.slots).push(slot);
    }
}

pub(crate) fn capture_registry() -> Result<EmergencyRegistry> {
    REGISTRY
        .get_or_init(start_registry)
        .as_ref()
        .map(Clone::clone)
        .map_err(|detail| anyhow::anyhow!(detail.clone()))
}

pub(crate) fn start_registry() -> std::result::Result<EmergencyRegistry, String> {
    let slots = Arc::new(Mutex::new(Vec::new()));
    let dispatch_slots = Arc::clone(&slots);
    let handle = std::thread::Builder::new()
        .name(DISPATCHER_THREAD_NAME.to_owned())
        .spawn(move || runtime::dispatch(dispatch_slots))
        .context(DISPATCHER_SPAWN_CONTEXT)
        .map_err(crate::worker::supervision::error_detail)?;
    Ok(EmergencyRegistry {
        slots,
        dispatcher: handle.thread().clone(),
    })
}

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
