#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(test)]
use std::sync::{Condvar, Mutex};

#[cfg(test)]
pub(super) const INJECTED_EXIT_PERSIST_FAILURE: &str = "injected exit persistence failure";
#[cfg(test)]
pub(super) const INJECTED_TERMINAL_PARSER_PANIC: &str = "injected terminal parser panic";

#[cfg(test)]
#[derive(Default)]
pub(super) struct AsyncControlHook {
    armed: AtomicBool,
    reached: tokio::sync::Notify,
    release: tokio::sync::Notify,
}

#[cfg(test)]
#[derive(Default)]
pub(super) struct ObservationHook {
    reached: AtomicBool,
    changed: tokio::sync::Notify,
}

#[cfg(test)]
impl ObservationHook {
    pub(super) fn mark_reached(&self) {
        self.reached.store(true, Ordering::Release);
        self.changed.notify_one();
    }

    pub(super) async fn wait_until_reached(&self) {
        while !self.reached.load(Ordering::Acquire) {
            self.changed.notified().await;
        }
    }
}

#[cfg(test)]
impl AsyncControlHook {
    pub(super) fn arm(&self) {
        self.armed.store(true, Ordering::Release);
    }

    pub(super) async fn pause_if_armed(&self) {
        if !self.armed.swap(false, Ordering::AcqRel) {
            return;
        }
        self.reached.notify_one();
        self.release.notified().await;
    }

    pub(super) async fn wait_until_reached(&self) {
        self.reached.notified().await;
    }

    pub(super) fn resume(&self) {
        self.release.notify_one();
    }
}

#[cfg(test)]
#[derive(Debug, Default)]
pub(super) struct BlockingControlHook {
    armed: AtomicBool,
    state: Mutex<BlockingHookState>,
    changed: Condvar,
}

#[cfg(test)]
#[derive(Debug, Default)]
struct BlockingHookState {
    reached: bool,
    released: bool,
}

#[cfg(test)]
impl BlockingControlHook {
    pub(super) fn arm(&self) {
        self.armed.store(true, Ordering::Release);
    }

    pub(super) fn pause_if_armed(&self) {
        if !self.armed.swap(false, Ordering::AcqRel) {
            return;
        }
        let mut state = self.state.lock().unwrap();
        state.reached = true;
        self.changed.notify_all();
        while !state.released {
            state = self.changed.wait(state).unwrap();
        }
    }

    pub(super) fn wait_until_reached(&self) {
        let mut state = self.state.lock().unwrap();
        while !state.reached {
            state = self.changed.wait(state).unwrap();
        }
    }

    pub(super) fn is_reached(&self) -> bool {
        self.state.lock().unwrap().reached
    }

    pub(super) fn resume(&self) {
        let mut state = self.state.lock().unwrap();
        state.released = true;
        self.changed.notify_all();
    }
}

#[cfg(test)]
#[derive(Default)]
pub(super) struct ControlTestHooks {
    controller_persist_failure: AtomicBool,
    controller_persist_indeterminate: AtomicBool,
    custody_after_runtime_publish: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    exit_persist_blocked: AtomicBool,
    terminal_parser_panic: AtomicBool,
    pub(super) controller_before_gate: AsyncControlHook,
    pub(super) attachment_after_subscribe: AsyncControlHook,
    pub(super) input_before_delivery: AsyncControlHook,
    pub(super) signal_before_delivery: AsyncControlHook,
    pub(super) terminate_before_delivery: AsyncControlHook,
    pub(super) termination_after_tombstone: AsyncControlHook,
    pub(super) custody_before_promotion: AsyncControlHook,
    pub(super) custody_before_activation: AsyncControlHook,
    pub(super) custody_after_unavailable: AsyncControlHook,
    pub(super) alias_closure_waiter_queued: ObservationHook,
    pub(super) system_mutation_before_gate: ObservationHook,
    pub(super) alias_closure_before_try_admission: BlockingControlHook,
    pub(super) finalizer_before_shutdown: BlockingControlHook,
    pub(super) response_before_admission: AsyncControlHook,
    pub(super) response_before_write: AsyncControlHook,
    pub(super) runtime_before_cleanup: AsyncControlHook,
    pub(super) owner_fault_before_cleanup: BlockingControlHook,
    pub(super) snapshot_before_publication: BlockingControlHook,
    pub(super) resize_after_dimensions: BlockingControlHook,
    pub(super) parser_recovery_after_dimensions: BlockingControlHook,
}

#[cfg(test)]
impl ControlTestHooks {
    pub(super) fn fail_next_controller_persist(&self) {
        self.controller_persist_failure
            .store(true, Ordering::Release);
    }

    pub(super) fn take_controller_persist_failure(&self) -> bool {
        self.controller_persist_failure
            .swap(false, Ordering::AcqRel)
    }

    pub(super) fn make_next_controller_persist_indeterminate(&self) {
        self.controller_persist_indeterminate
            .store(true, Ordering::Release);
    }

    pub(super) fn take_controller_persist_indeterminate(&self) -> bool {
        self.controller_persist_indeterminate
            .swap(false, Ordering::AcqRel)
    }

    pub(super) fn after_next_custody_runtime_publish(&self, hook: impl FnOnce() + Send + 'static) {
        *self.custody_after_runtime_publish.lock().unwrap() = Some(Box::new(hook));
    }

    pub(super) fn run_after_custody_runtime_publish(&self) {
        let hook = self.custody_after_runtime_publish.lock().unwrap().take();
        if let Some(hook) = hook {
            hook();
        }
    }

    pub(super) fn block_exit_persist(&self) {
        self.exit_persist_blocked.store(true, Ordering::Release);
    }

    pub(super) fn allow_exit_persist(&self) {
        self.exit_persist_blocked.store(false, Ordering::Release);
    }

    pub(super) fn exit_persist_is_blocked(&self) -> bool {
        self.exit_persist_blocked.load(Ordering::Acquire)
    }

    pub(super) fn panic_next_terminal_parse(&self) {
        self.terminal_parser_panic.store(true, Ordering::Release);
    }

    pub(super) fn take_terminal_parser_panic(&self) -> bool {
        self.terminal_parser_panic.swap(false, Ordering::AcqRel)
    }
}
