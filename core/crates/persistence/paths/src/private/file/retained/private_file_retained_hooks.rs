use anyhow::Result;

#[cfg(any(test, feature = "persistence-test-support"))]
type FallibleHook = Option<Box<dyn FnOnce() -> Result<()>>>;

#[cfg(any(test, feature = "persistence-test-support"))]
thread_local! {
    static ISOLATION_HOOK: std::cell::RefCell<FallibleHook> = const { std::cell::RefCell::new(None) };
    static INSTALL_HOOK: std::cell::RefCell<FallibleHook> = const { std::cell::RefCell::new(None) };
    static RENAME_HOOK: std::cell::RefCell<FallibleHook> = const { std::cell::RefCell::new(None) };
    static RESTORE_HOOK: std::cell::RefCell<FallibleHook> = const { std::cell::RefCell::new(None) };
}

#[cfg(any(test, feature = "persistence-test-support"))]
pub(crate) fn set_retained_exchange_hook<F: FnOnce() -> Result<()> + 'static>(hook: F) {
    ISOLATION_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(any(test, feature = "persistence-test-support"))]
pub(crate) fn set_retained_install_hook<F: FnOnce() -> Result<()> + 'static>(hook: F) {
    INSTALL_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(any(test, feature = "persistence-test-support"))]
pub(crate) fn set_retained_rename_hook<F: FnOnce() -> Result<()> + 'static>(hook: F) {
    RENAME_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(any(test, feature = "persistence-test-support"))]
pub(crate) fn set_retained_restore_hook<F: FnOnce() -> Result<()> + 'static>(hook: F) {
    RESTORE_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

pub(super) fn run_isolation_hook() -> Result<()> {
    run(Hook::Isolation)
}

pub(super) fn run_install_hook() -> Result<()> {
    run(Hook::Install)
}

pub(super) fn run_rename_hook() -> Result<()> {
    run(Hook::Rename)
}

pub(super) fn run_restore_hook() -> Result<()> {
    run(Hook::Restore)
}

enum Hook {
    Isolation,
    Install,
    Rename,
    Restore,
}

#[cfg(any(test, feature = "persistence-test-support"))]
fn run(hook: Hook) -> Result<()> {
    match hook {
        Hook::Isolation => ISOLATION_HOOK.with(run_slot),
        Hook::Install => INSTALL_HOOK.with(run_slot),
        Hook::Rename => RENAME_HOOK.with(run_slot),
        Hook::Restore => RESTORE_HOOK.with(run_slot),
    }
}

#[cfg(not(any(test, feature = "persistence-test-support")))]
fn run(hook: Hook) -> Result<()> {
    let _ = hook;
    Ok(())
}

#[cfg(any(test, feature = "persistence-test-support"))]
fn run_slot(slot: &std::cell::RefCell<FallibleHook>) -> Result<()> {
    match slot.borrow_mut().take() {
        Some(hook) => hook(),
        None => Ok(()),
    }
}
