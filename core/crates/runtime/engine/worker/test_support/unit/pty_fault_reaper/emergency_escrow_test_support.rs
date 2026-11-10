use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, Weak};
use std::{io, time::Duration};

const HOOK_WAIT_TIMEOUT: Duration = Duration::from_secs(10);
const OWNER_FAILURE_WAIT_DETAIL: &str = "emergency owner spawn failure was not observed";
const SLOT_DROP_WAIT_DETAIL: &str = "emergency escrow slot was not pruned";
const SINGLE_OWNER_SPAWN_FAILURE: usize = 1;
const FAILURE_OBSERVATION_INCREMENT: usize = 1;
const EMERGENCY_PRODUCER_MISSING_DETAIL: &str = "emergency escrow producer is reserved";
const EMERGENCY_SLOT_MISSING_DETAIL: &str = "emergency escrow slot is reserved";
const DRIVE_SCHEDULE_WAIT_DETAIL: &str = "emergency owner drive schedule was not observed";

#[derive(Default)]
pub(super) struct EmergencySlotTestHooks {
    shared: Arc<HookShared>,
}

#[derive(Default)]
struct HookShared {
    state: Mutex<HookState>,
    changed: Condvar,
    block_install_return: AtomicBool,
}

#[derive(Default)]
struct HookState {
    owner_spawn_fault: Option<InjectedOwnerSpawnFault>,
    owner_spawn_failures_remaining: usize,
    owner_spawn_failures_observed: usize,
    block_owner_retry: bool,
    allow_owner_retry: bool,
    install_publication_observed: bool,
    allow_install_return: bool,
    drive_schedule_prompt: Option<bool>,
    slot_dropped: bool,
}

#[derive(Clone, Copy)]
enum InjectedOwnerSpawnFault {
    Error,
    Panic,
}

pub(super) struct EmergencyEscrowProbe {
    slot: Weak<EmergencySlot>,
    shared: Arc<HookShared>,
}

impl EmergencySlotTestHooks {
    pub(super) fn inject_owner_spawn_fault(&self) -> io::Result<()> {
        let fault = {
            let mut state = lock(&self.shared.state);
            if state.owner_spawn_failures_remaining == usize::default() {
                return Ok(());
            }
            state.owner_spawn_failures_remaining -= FAILURE_OBSERVATION_INCREMENT;
            state.owner_spawn_fault
        };
        match fault {
            Some(InjectedOwnerSpawnFault::Error) => {
                Err(io::Error::other(INJECTED_OWNER_SPAWN_FAILURE_DETAIL))
            }
            Some(InjectedOwnerSpawnFault::Panic) => {
                panic!("{INJECTED_OWNER_SPAWN_PANIC_DETAIL}")
            }
            None => Ok(()),
        }
    }

    pub(super) fn record_owner_spawn_failure_and_wait(&self) {
        let mut state = lock(&self.shared.state);
        state.owner_spawn_failures_observed = state
            .owner_spawn_failures_observed
            .saturating_add(FAILURE_OBSERVATION_INCREMENT);
        self.shared.changed.notify_all();
        while state.block_owner_retry && !state.allow_owner_retry {
            state = self
                .shared
                .changed
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }

    pub(super) fn wait_before_install_return(&self) {
        if !self.shared.block_install_return.load(Ordering::Acquire) {
            return;
        }
        let mut state = lock(&self.shared.state);
        state.install_publication_observed = true;
        self.shared.changed.notify_all();
        while !state.allow_install_return {
            state = self
                .shared
                .changed
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }

    pub(super) fn record_drive_schedule(&self, prompt: bool) {
        lock(&self.shared.state).drive_schedule_prompt = Some(prompt);
        self.shared.changed.notify_all();
    }

    fn record_slot_drop(&self) {
        lock(&self.shared.state).slot_dropped = true;
        self.shared.changed.notify_all();
    }
}

impl EmergencyEscrowProducer {
    pub(super) fn probe_for_test(&self) -> EmergencyEscrowProbe {
        let slot = self.slot.as_ref().expect(EMERGENCY_SLOT_MISSING_DETAIL);
        EmergencyEscrowProbe {
            slot: Arc::downgrade(slot),
            shared: Arc::clone(&slot.test_hooks.shared),
        }
    }

    pub(super) fn reserve_in_for_test(
        observer: FaultReapStateObserver,
        registry: EmergencyRegistry,
    ) -> Self {
        let slot = Arc::new(EmergencySlot {
            phase: AtomicU8::new(EmergencyPhase::Vacant as u8),
            envelope: UnsafeCell::new(MaybeUninit::uninit()),
            dispatcher: registry.dispatcher.clone(),
            test_hooks: EmergencySlotTestHooks::default(),
        });
        registry.register(Arc::clone(&slot));
        Self {
            slot: Some(slot),
            control: Some(super::super::fault_reap_construction::vacant_fault_reap(
                observer,
                super::super::schedule::FaultReapSchedule::production(),
            )),
            completion: Some(FaultReapCompletion::log()),
        }
    }
}

impl FaultReaperReservation {
    pub(super) fn emergency_probe_for_test(&self) -> EmergencyEscrowProbe {
        self.emergency
            .as_ref()
            .expect(EMERGENCY_PRODUCER_MISSING_DETAIL)
            .probe_for_test()
    }
}

impl Drop for EmergencySlot {
    fn drop(&mut self) {
        self.test_hooks.record_slot_drop();
    }
}

include!("emergency_escrow_probe_test_support.rs");
