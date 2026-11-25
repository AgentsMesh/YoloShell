use super::*;
use std::cell::RefCell;

const HOOK_ALREADY_INSTALLED: &str = "private temporary race hook is already installed";

type Hook = Box<dyn FnOnce() -> Result<()>>;

thread_local! {
    static AFTER_STAGE_IDENTITY: RefCell<Option<Hook>> = RefCell::new(None);
    static BEFORE_RETRY: RefCell<Option<Hook>> = RefCell::new(None);
    static BEFORE_TARGET_OPEN: RefCell<Option<Hook>> = RefCell::new(None);
}

pub(crate) struct TemporaryRaceHookGuard;

impl PrivateFileEntry {
    pub(crate) fn install_after_stage_identity_hook(
        &self,
        hook: impl FnOnce() -> Result<()> + 'static,
    ) -> TemporaryRaceHookGuard {
        install(&AFTER_STAGE_IDENTITY, hook)
    }

    pub(crate) fn install_before_target_open_hook(
        &self,
        hook: impl FnOnce() -> Result<()> + 'static,
    ) -> TemporaryRaceHookGuard {
        install(&BEFORE_TARGET_OPEN, hook)
    }

    pub(crate) fn install_before_temporary_retry_hook(
        &self,
        hook: impl FnOnce() -> Result<()> + 'static,
    ) -> TemporaryRaceHookGuard {
        install(&BEFORE_RETRY, hook)
    }

    pub(crate) fn temporary_with_hook<F>(
        &self,
        prefix: &str,
        suffix: &str,
        hook: F,
    ) -> Result<TemporaryPrivateFile>
    where
        F: FnMut() -> Result<()>,
    {
        self.temporary_with(prefix, suffix, || Ok(()), || Ok(()), hook)
    }

    pub(crate) fn temporary_with_open_hook<F>(
        &self,
        prefix: &str,
        suffix: &str,
        hook: F,
    ) -> Result<TemporaryPrivateFile>
    where
        F: FnMut() -> Result<()>,
    {
        self.temporary_with(prefix, suffix, || Ok(()), hook, || Ok(()))
    }

    pub(crate) fn temporary_with_target_open_hook<F>(
        &self,
        prefix: &str,
        suffix: &str,
        hook: F,
    ) -> Result<TemporaryPrivateFile>
    where
        F: FnMut() -> Result<()>,
    {
        self.temporary_with(prefix, suffix, hook, || Ok(()), || Ok(()))
    }
}

pub(super) fn run_after_stage_identity() -> Result<()> {
    run(&AFTER_STAGE_IDENTITY)
}

pub(super) fn run_before_target_open() -> Result<()> {
    run(&BEFORE_TARGET_OPEN)
}

pub(super) fn run_before_retry() -> Result<()> {
    run(&BEFORE_RETRY)
}

fn install(
    slot: &'static std::thread::LocalKey<RefCell<Option<Hook>>>,
    hook: impl FnOnce() -> Result<()> + 'static,
) -> TemporaryRaceHookGuard {
    slot.with(|slot| {
        let previous = slot.borrow_mut().replace(Box::new(hook));
        assert!(previous.is_none(), "{HOOK_ALREADY_INSTALLED}");
    });
    TemporaryRaceHookGuard
}

fn run(slot: &'static std::thread::LocalKey<RefCell<Option<Hook>>>) -> Result<()> {
    match slot.with(|slot| slot.borrow_mut().take()) {
        Some(hook) => hook(),
        None => Ok(()),
    }
}

impl Drop for TemporaryRaceHookGuard {
    fn drop(&mut self) {
        AFTER_STAGE_IDENTITY.with(|slot| *slot.borrow_mut() = None);
        BEFORE_RETRY.with(|slot| *slot.borrow_mut() = None);
        BEFORE_TARGET_OPEN.with(|slot| *slot.borrow_mut() = None);
    }
}
