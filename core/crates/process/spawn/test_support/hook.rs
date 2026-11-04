use std::cell::Cell;

const NESTED_HOOK_DETAIL: &str = "spawn gate rejection hook is already installed";
const REJECTED_ENTRY_DETAIL: &str = "spawn gate was reacquired inside an existing boundary";
pub const NORMALIZATION_FAILURE_DETAIL: &str =
    "injected SIGCHLD waitability normalization failure";

thread_local! {
    static REJECT_ENTRY: Cell<bool> = const { Cell::new(false) };
    static FAIL_NORMALIZATION: Cell<bool> = const { Cell::new(false) };
}

pub struct RejectSpawnGateEntry;
pub struct FailSigchldNormalization;

pub fn reject_spawn_gate_entry() -> RejectSpawnGateEntry {
    REJECT_ENTRY.with(|reject| {
        assert!(!reject.replace(true), "{NESTED_HOOK_DETAIL}");
    });
    RejectSpawnGateEntry
}

pub(crate) fn reject_gate_entry() {
    REJECT_ENTRY.with(|reject| {
        assert!(!reject.get(), "{REJECTED_ENTRY_DETAIL}");
    });
}

pub fn fail_sigchld_normalization() -> FailSigchldNormalization {
    FAIL_NORMALIZATION.with(|fail| {
        assert!(!fail.replace(true), "{NESTED_HOOK_DETAIL}");
    });
    FailSigchldNormalization
}

pub(crate) fn inject_normalization_failure() -> anyhow::Result<()> {
    if FAIL_NORMALIZATION.with(Cell::get) {
        return Err(anyhow::anyhow!(NORMALIZATION_FAILURE_DETAIL));
    }
    Ok(())
}

impl Drop for RejectSpawnGateEntry {
    fn drop(&mut self) {
        REJECT_ENTRY.with(|reject| reject.set(false));
    }
}

impl Drop for FailSigchldNormalization {
    fn drop(&mut self) {
        FAIL_NORMALIZATION.with(|fail| fail.set(false));
    }
}
