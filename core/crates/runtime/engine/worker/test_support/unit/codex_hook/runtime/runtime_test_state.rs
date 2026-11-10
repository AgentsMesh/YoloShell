use super::CodexHookRuntime;
use std::sync::atomic::AtomicBool;

pub(in crate::worker::codex_hook) type RuntimeTestKey = uuid::Uuid;

pub(super) struct RuntimeFaults {
    pub(super) key: RuntimeTestKey,
    pub(super) ingest: AtomicBool,
    pub(super) reconcile: AtomicBool,
    pub(super) process_exit: AtomicBool,
}

impl Default for RuntimeFaults {
    fn default() -> Self {
        Self {
            key: RuntimeTestKey::new_v4(),
            ingest: AtomicBool::new(false),
            reconcile: AtomicBool::new(false),
            process_exit: AtomicBool::new(false),
        }
    }
}

impl CodexHookRuntime {
    pub(in crate::worker::codex_hook) fn test_key(&self) -> RuntimeTestKey {
        self.faults.key
    }
}

impl Drop for RuntimeFaults {
    fn drop(&mut self) {
        super::test_support::clear_runtime_state(self.key);
        super::ack_test_support::clear_runtime_state(self.key);
        super::intake_test_gate::clear_runtime_state(self.key);
        crate::worker::codex_hook::intake_task::clear_test_faults(self.key);
    }
}
