use super::{CodexHookRuntime, RuntimeTestKey};
use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::Duration;

const INTAKE_GATE_WAIT: Duration = crate::session_timing::WORKER_READY_TIMEOUT;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
enum IntakeGateStage {
    Authorized,
    Folding,
}

pub(in crate::worker::codex_hook) struct IntakeGate {
    state: Mutex<IntakeGateState>,
    changed: Condvar,
}

#[derive(Default)]
struct IntakeGateState {
    entered: bool,
    released: bool,
}

impl IntakeGate {
    pub(in crate::worker::codex_hook) fn wait_until_entered(&self) {
        let state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let (state, timeout) = self
            .changed
            .wait_timeout_while(state, INTAKE_GATE_WAIT, |state| !state.entered)
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert!(state.entered && !timeout.timed_out());
    }

    pub(in crate::worker::codex_hook) fn release(&self) {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .released = true;
        self.changed.notify_all();
    }

    fn block(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.entered = true;
        self.changed.notify_all();
        while !state.released {
            state = self
                .changed
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }
}

impl CodexHookRuntime {
    pub(in crate::worker::codex_hook) fn gate_next_intake_authorization_for_test(
        &self,
    ) -> Arc<IntakeGate> {
        self.install_intake_gate(IntakeGateStage::Authorized)
    }

    pub(in crate::worker::codex_hook) fn gate_next_intake_fold_for_test(&self) -> Arc<IntakeGate> {
        self.install_intake_gate(IntakeGateStage::Folding)
    }

    pub(super) fn wait_for_intake_authorization_gate_for_test(&self) {
        self.wait_for_intake_gate(IntakeGateStage::Authorized);
    }

    pub(super) fn wait_for_intake_fold_gate_for_test(&self) {
        self.wait_for_intake_gate(IntakeGateStage::Folding);
    }

    fn install_intake_gate(&self, stage: IntakeGateStage) -> Arc<IntakeGate> {
        let gate = Arc::new(IntakeGate {
            state: Mutex::new(IntakeGateState::default()),
            changed: Condvar::new(),
        });
        let prior = gates()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert((self.test_key(), stage), Arc::clone(&gate));
        assert!(prior.is_none());
        gate
    }

    fn wait_for_intake_gate(&self, stage: IntakeGateStage) {
        let gate = gates()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&(self.test_key(), stage));
        if let Some(gate) = gate {
            gate.block();
        }
    }
}

fn gates() -> &'static Mutex<HashMap<(RuntimeTestKey, IntakeGateStage), Arc<IntakeGate>>> {
    static GATES: OnceLock<Mutex<HashMap<(RuntimeTestKey, IntakeGateStage), Arc<IntakeGate>>>> =
        OnceLock::new();
    GATES.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(super) fn clear_runtime_state(key: RuntimeTestKey) {
    gates()
        .lock()
        .unwrap()
        .retain(|(runtime, _), _| *runtime != key);
}
