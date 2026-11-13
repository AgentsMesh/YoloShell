use crate::agent::hooks::{HookIdentity, ProcessIdentity as HookProcessIdentity, ProviderVersion};
use anyhow::{Result, ensure};
use uuid::Uuid;

pub(super) fn validate_binding(
    terminal_id: Uuid,
    runtime_epoch: Uuid,
    shell: crate::process::ProcessIdentity,
    owner: &HookIdentity,
    provider_version: ProviderVersion,
) -> Result<()> {
    use super::super::constants::*;
    ensure!(
        !terminal_id.is_nil() && owner.terminal_id == terminal_id,
        CODEX_SESSION_ERROR
    );
    ensure!(
        !runtime_epoch.is_nil() && owner.runtime_epoch == runtime_epoch,
        CODEX_RUNTIME_ERROR
    );
    ensure!(
        HookProcessIdentity::new(shell.pid, shell.start_time).is_valid(),
        CODEX_PROCESS_ERROR
    );
    ensure!(
        provider_version == crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
        CODEX_VERSION_ERROR
    );
    owner.validate().map_err(anyhow::Error::new)
}

#[cfg(test)]
thread_local! {
    static BEFORE_INITIAL_PUBLISH_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(in crate::worker::codex_hook) fn set_before_initial_publish_hook(
    hook: impl FnOnce() + 'static,
) {
    BEFORE_INITIAL_PUBLISH_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(test)]
pub(super) fn run_before_initial_publish_hook() {
    let hook = BEFORE_INITIAL_PUBLISH_HOOK.with(|slot| slot.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
    }
}

#[cfg(not(test))]
pub(super) fn run_before_initial_publish_hook() {}
