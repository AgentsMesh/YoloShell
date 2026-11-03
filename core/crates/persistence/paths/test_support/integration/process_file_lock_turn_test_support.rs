use super::*;

thread_local! {
    static REGISTRY_ADMISSION_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
}

impl ProcessLockTurn {
    pub(crate) fn queued_waiters_for_test(key: ObjectIdentity) -> usize {
        current_registry()
            .turns
            .get(&key)
            .map_or(usize::default(), |state| state.waiters.len())
    }

    pub(in crate::paths::process_file_lock) fn set_registry_admission_hook(
        admission_hook: impl FnOnce() + 'static,
    ) {
        REGISTRY_ADMISSION_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(admission_hook)));
    }

    pub(in crate::paths::process_file_lock) fn release_before_notification_for_test(
        self,
    ) -> Arc<wait_signal::ProcessWaitSignal> {
        let retained = std::mem::ManuallyDrop::new(self);
        release_state(retained.key, retained.process_id).expect(TURN_STATE_MISSING)
    }

    pub(in crate::paths::process_file_lock) fn release_at_before_notification_for_test(
        self,
        occurred_at: Instant,
    ) -> Arc<wait_signal::ProcessWaitSignal> {
        let retained = std::mem::ManuallyDrop::new(self);
        let mut registry = current_registry();
        release_state_at(
            &mut registry,
            retained.key,
            retained.process_id,
            occurred_at,
        )
        .expect(TURN_STATE_MISSING)
    }
}

pub(super) fn run_registry_admission_hook() {
    REGISTRY_ADMISSION_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook();
        }
    });
}

pub(in crate::paths::process_file_lock) fn arm_front_renewal_overflow_for_test(
    key: ObjectIdentity,
) {
    let mut registry = current_registry();
    let state = registry.turns.get_mut(&key).expect(TURN_STATE_MISSING);
    state
        .waiters
        .front_mut()
        .expect(TURN_STATE_MISSING)
        .arm_renewal_overflow_for_test();
}

pub(in crate::paths::process_file_lock) fn expire_front_waiter_for_test(key: ObjectIdentity) {
    let signal = {
        let mut registry = current_registry();
        let state = registry.turns.get_mut(&key).expect(TURN_STATE_MISSING);
        state
            .waiters
            .front_mut()
            .expect(TURN_STATE_MISSING)
            .expire_for_test();
        Arc::clone(&state.signal)
    };
    signal.notify_all();
}

pub(in crate::paths::process_file_lock) fn notify_waiters_for_test(
    signal: &Arc<wait_signal::ProcessWaitSignal>,
) {
    signal.notify_all();
}

pub(in crate::paths::process_file_lock) fn insert_unowned_waiter_for_test(
    key: ObjectIdentity,
) -> Arc<()> {
    let waiter = waiter::TurnWaiter::absolute(Instant::now());
    let identity = waiter.identity();
    let mut state = TurnState::default();
    state.waiters.push_back(waiter);
    current_registry().turns.insert(key, state);
    identity
}
