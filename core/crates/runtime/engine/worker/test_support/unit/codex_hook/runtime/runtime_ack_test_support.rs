use super::{CodexHookRuntime, RuntimeTestKey};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

pub(in crate::worker::codex_hook) struct AckWriteGate {
    state: Arc<AckWriteGateState>,
}

struct AckWriteGateState {
    entered: tokio::sync::Notify,
    released: AtomicBool,
    changed: tokio::sync::Notify,
}

fn gates() -> &'static Mutex<HashMap<RuntimeTestKey, Arc<AckWriteGateState>>> {
    static GATES: OnceLock<Mutex<HashMap<RuntimeTestKey, Arc<AckWriteGateState>>>> =
        OnceLock::new();
    GATES.get_or_init(Default::default)
}

impl CodexHookRuntime {
    pub(in crate::worker::codex_hook) fn gate_next_ack_write_for_test(&self) -> AckWriteGate {
        let state = Arc::new(AckWriteGateState {
            entered: tokio::sync::Notify::new(),
            released: AtomicBool::new(false),
            changed: tokio::sync::Notify::new(),
        });
        let previous = gates()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(self.test_key(), Arc::clone(&state));
        assert!(previous.is_none());
        AckWriteGate { state }
    }

    pub(in crate::worker::codex_hook) async fn wait_for_ack_write_gate_for_test(&self) {
        let gate = gates()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&self.test_key());
        if let Some(gate) = gate {
            gate.block().await;
        }
    }
}

impl AckWriteGate {
    pub(in crate::worker::codex_hook) async fn wait_until_entered(&self) {
        self.state.entered.notified().await;
    }

    pub(in crate::worker::codex_hook) fn release(&self) {
        self.state.release();
    }
}

impl AckWriteGateState {
    async fn block(&self) {
        self.entered.notify_waiters();
        while !self.released.load(Ordering::Acquire) {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if self.released.load(Ordering::Acquire) {
                break;
            }
            changed.await;
        }
    }

    fn release(&self) {
        self.released.store(true, Ordering::Release);
        self.changed.notify_waiters();
    }
}

impl Drop for AckWriteGate {
    fn drop(&mut self) {
        self.state.release();
    }
}

pub(super) fn clear_runtime_state(key: RuntimeTestKey) {
    gates().lock().unwrap().remove(&key);
}
