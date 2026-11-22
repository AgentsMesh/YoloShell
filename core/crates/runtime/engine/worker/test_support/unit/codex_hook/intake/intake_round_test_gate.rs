use super::super::super::router::{CodexSessionRouter, RouterTestKey};
use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex, OnceLock};

pub(in crate::worker::codex_hook) struct IntakeRoundGate {
    state: Mutex<GateState>,
    changed: Condvar,
}

#[derive(Default)]
struct GateState {
    entered: bool,
    released: bool,
}

impl CodexSessionRouter {
    pub(in crate::worker::codex_hook) async fn pause_next_intake_round_for_test(
        &self,
    ) -> impl Drop {
        let gate = self.gate_next_intake_round_for_test();
        let entered = Arc::clone(&gate);
        let pause = IntakeRoundPause {
            gate,
            key: router_key(self),
        };
        tokio::task::spawn_blocking(move || entered.wait_until_entered())
            .await
            .unwrap();
        pause
    }

    pub(in crate::worker::codex_hook) fn gate_next_intake_round_for_test(
        &self,
    ) -> Arc<IntakeRoundGate> {
        let gate = Arc::new(IntakeRoundGate {
            state: Mutex::default(),
            changed: Condvar::new(),
        });
        let previous = gates()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(router_key(self), Arc::clone(&gate));
        assert!(previous.is_none());
        gate
    }
}

struct IntakeRoundPause {
    gate: Arc<IntakeRoundGate>,
    key: RouterTestKey,
}

impl Drop for IntakeRoundPause {
    fn drop(&mut self) {
        let mut pending = gates()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if pending
            .get(&self.key)
            .is_some_and(|gate| Arc::ptr_eq(gate, &self.gate))
        {
            pending.remove(&self.key);
        }
        drop(pending);
        self.gate.release();
    }
}

impl IntakeRoundGate {
    pub(in crate::worker::codex_hook) fn wait_until_entered(&self) {
        let state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        drop(
            self.changed
                .wait_while(state, |state| !state.entered && !state.released)
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        );
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
        drop(
            self.changed
                .wait_while(state, |state| !state.released)
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        );
    }
}

pub(super) fn wait_after_enumeration(router: &CodexSessionRouter) {
    let gate = gates()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(&router_key(router));
    if let Some(gate) = gate {
        gate.block();
    }
}

fn gates() -> &'static Mutex<HashMap<RouterTestKey, Arc<IntakeRoundGate>>> {
    static GATES: OnceLock<Mutex<HashMap<RouterTestKey, Arc<IntakeRoundGate>>>> = OnceLock::new();
    GATES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn router_key(router: &CodexSessionRouter) -> RouterTestKey {
    router.test_key()
}
