use super::{CodexHookRuntime, RuntimeTestKey};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier, Mutex, OnceLock};
use tokio::sync::Notify;

const HANDOFF_GATE_PARTICIPANTS: usize = 2;

pub(in crate::worker::codex_hook) struct HandoffCancellationGate {
    entered: Barrier,
    released: Barrier,
}

pub(in crate::worker::codex_hook) struct ResponseCompletionGate {
    entered: Notify,
    released: Notify,
}

impl ResponseCompletionGate {
    pub(in crate::worker::codex_hook) async fn wait_until_entered(&self) {
        self.entered.notified().await;
    }

    pub(in crate::worker::codex_hook) fn release(&self) {
        self.released.notify_one();
    }

    async fn block(&self) {
        self.entered.notify_one();
        self.released.notified().await;
    }
}

impl HandoffCancellationGate {
    pub(in crate::worker::codex_hook) fn wait_until_entered(&self) {
        self.entered.wait();
    }

    pub(in crate::worker::codex_hook) fn release(&self) {
        self.released.wait();
    }

    fn block(&self) {
        self.entered.wait();
        self.released.wait();
    }
}

fn handoff_gates() -> &'static Mutex<HashMap<RuntimeTestKey, Arc<HandoffCancellationGate>>> {
    static GATES: OnceLock<Mutex<HashMap<RuntimeTestKey, Arc<HandoffCancellationGate>>>> =
        OnceLock::new();
    GATES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn response_gates() -> &'static Mutex<HashMap<RuntimeTestKey, Arc<ResponseCompletionGate>>> {
    static GATES: OnceLock<Mutex<HashMap<RuntimeTestKey, Arc<ResponseCompletionGate>>>> =
        OnceLock::new();
    GATES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn commit_settlement_gates() -> &'static Mutex<HashMap<RuntimeTestKey, Arc<HandoffCancellationGate>>>
{
    static GATES: OnceLock<Mutex<HashMap<RuntimeTestKey, Arc<HandoffCancellationGate>>>> =
        OnceLock::new();
    GATES.get_or_init(|| Mutex::new(HashMap::new()))
}

impl CodexHookRuntime {
    pub(in crate::worker::codex_hook) fn storage_failed(&self) -> Result<bool> {
        Ok(self.lock_lifecycle_anyhow()?.storage_failed())
    }

    pub(in crate::worker::codex_hook) fn poison_lifecycle_for_test(&self) {
        let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _lifecycle = self.registration.control().lifecycle.lock().unwrap();
            std::panic::panic_any(());
        }));
        assert!(poisoned.is_err());
    }

    pub(in crate::worker::codex_hook) fn panic_next_ingest_for_test(&self) {
        self.faults.ingest.store(true, Ordering::SeqCst);
    }

    pub(in crate::worker::codex_hook) fn panic_next_reconcile_for_test(&self) {
        self.faults.reconcile.store(true, Ordering::SeqCst);
    }

    pub(in crate::worker::codex_hook) fn panic_next_process_exit_for_test(&self) {
        self.faults.process_exit.store(true, Ordering::SeqCst);
    }

    pub(in crate::worker::codex_hook) fn exhaust_active_authority_for_test(&self) {
        self.lock_lifecycle_anyhow()
            .unwrap()
            .exhaust_active_authority_for_test();
    }

    pub(in crate::worker::codex_hook) fn gate_next_handoff_for_test(
        &self,
    ) -> Arc<HandoffCancellationGate> {
        let gate = Arc::new(HandoffCancellationGate {
            entered: Barrier::new(HANDOFF_GATE_PARTICIPANTS),
            released: Barrier::new(HANDOFF_GATE_PARTICIPANTS),
        });
        let previous = handoff_gates()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(self.test_key(), Arc::clone(&gate));
        assert!(previous.is_none());
        gate
    }

    pub(in crate::worker::codex_hook) fn wait_for_handoff_gate_for_test(&self) {
        let gate = handoff_gates()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&self.test_key());
        if let Some(gate) = gate {
            gate.block();
        }
    }

    pub(in crate::worker::codex_hook) fn gate_next_commit_settlement_for_test(
        &self,
    ) -> Arc<HandoffCancellationGate> {
        let gate = Arc::new(HandoffCancellationGate {
            entered: Barrier::new(HANDOFF_GATE_PARTICIPANTS),
            released: Barrier::new(HANDOFF_GATE_PARTICIPANTS),
        });
        let previous = commit_settlement_gates()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(self.test_key(), Arc::clone(&gate));
        assert!(previous.is_none());
        gate
    }

    pub(in crate::worker::codex_hook) fn wait_for_commit_settlement_gate_for_test(&self) {
        let gate = commit_settlement_gates()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&self.test_key());
        if let Some(gate) = gate {
            gate.block();
        }
    }

    pub(in crate::worker::codex_hook) fn gate_next_response_for_test(
        &self,
    ) -> Arc<ResponseCompletionGate> {
        let gate = Arc::new(ResponseCompletionGate {
            entered: Notify::new(),
            released: Notify::new(),
        });
        let previous = response_gates()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(self.test_key(), Arc::clone(&gate));
        assert!(previous.is_none());
        gate
    }

    pub(in crate::worker::codex_hook) async fn wait_for_response_gate_for_test(&self) {
        let gate = response_gates()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&self.test_key());
        if let Some(gate) = gate {
            gate.block().await;
        }
    }

    pub(super) fn panic_ingest_for_test(&self) {
        panic_if_armed(&self.faults.ingest);
    }

    pub(super) fn panic_reconcile_for_test(&self) {
        panic_if_armed(&self.faults.reconcile);
    }

    pub(super) fn panic_process_exit_for_test(&self) {
        panic_if_armed(&self.faults.process_exit);
    }
}

fn panic_if_armed(fault: &AtomicBool) {
    if fault.swap(false, Ordering::SeqCst) {
        std::panic::panic_any(());
    }
}

pub(super) fn clear_runtime_state(key: RuntimeTestKey) {
    handoff_gates().lock().unwrap().remove(&key);
    response_gates().lock().unwrap().remove(&key);
    commit_settlement_gates().lock().unwrap().remove(&key);
}
