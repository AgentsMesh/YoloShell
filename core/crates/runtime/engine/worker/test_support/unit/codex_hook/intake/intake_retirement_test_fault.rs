use super::super::super::runtime::{CodexHookRuntime, RuntimeTestKey};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const INJECTED_RETIREMENT_FAILURE: &str = "injected Codex retirement coordinator failure";

type Hook = Box<dyn FnOnce() -> Result<()> + Send>;
type HookKey = (RuntimeTestKey, RetirementFaultStage);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(in crate::worker::codex_hook) enum RetirementFaultStage {
    AfterSealPublication,
    AfterTerminalSettlement,
    BeforeTerminalIntakeProof,
    AfterRetirementPublication,
}

impl CodexHookRuntime {
    pub(in crate::worker::codex_hook) fn fail_next_retirement_at_for_test(
        &self,
        stage: RetirementFaultStage,
    ) {
        self.hook_next_retirement_at_for_test(stage, || anyhow::bail!(INJECTED_RETIREMENT_FAILURE));
    }

    pub(in crate::worker::codex_hook) fn hook_next_retirement_at_for_test(
        &self,
        stage: RetirementFaultStage,
        hook: impl FnOnce() -> Result<()> + Send + 'static,
    ) {
        let previous = hooks()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert((self.test_key(), stage), Box::new(hook));
        assert!(previous.is_none());
    }
}

pub(super) fn run(runtime: &CodexHookRuntime, stage: RetirementFaultStage) -> Result<()> {
    let hook = hooks()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(&(runtime.test_key(), stage));
    match hook {
        Some(hook) => hook(),
        None => Ok(()),
    }
}

fn hooks() -> &'static Mutex<HashMap<HookKey, Hook>> {
    static HOOKS: OnceLock<Mutex<HashMap<HookKey, Hook>>> = OnceLock::new();
    HOOKS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(in crate::worker::codex_hook) fn clear_test_faults(key: RuntimeTestKey) {
    hooks()
        .lock()
        .unwrap()
        .retain(|(runtime, _), _| *runtime != key);
}
