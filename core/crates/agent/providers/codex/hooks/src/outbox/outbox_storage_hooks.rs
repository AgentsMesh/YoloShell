use std::path::Path;

#[cfg(any(test, feature = "runtime-component-tests"))]
thread_local! {
    static BEFORE_READ: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
    static AFTER_OPEN: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
    static AFTER_LOAD: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
    static AFTER_OPERATION: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(any(test, feature = "runtime-component-tests"))]
type PersistHook = Box<dyn FnOnce() + Send>;

#[cfg(any(test, feature = "runtime-component-tests"))]
fn persist_hooks()
-> &'static std::sync::Mutex<std::collections::HashMap<std::path::PathBuf, PersistHook>> {
    static HOOKS: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<std::path::PathBuf, PersistHook>>,
    > = std::sync::OnceLock::new();
    HOOKS.get_or_init(Default::default)
}

#[cfg(any(test, feature = "runtime-component-tests"))]
fn reproof_hooks()
-> &'static std::sync::Mutex<std::collections::HashMap<std::path::PathBuf, PersistHook>> {
    static HOOKS: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<std::path::PathBuf, PersistHook>>,
    > = std::sync::OnceLock::new();
    HOOKS.get_or_init(Default::default)
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub fn set_before_persist_commit_hook(
    path: std::path::PathBuf,
    hook: impl FnOnce() + Send + 'static,
) {
    let previous = persist_hooks()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(path, Box::new(hook));
    assert!(previous.is_none());
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub fn set_before_persist_reproof_hook(
    path: std::path::PathBuf,
    hook: impl FnOnce() + Send + 'static,
) {
    let previous = reproof_hooks()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(path, Box::new(hook));
    assert!(previous.is_none());
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(in crate::codex_hooks) fn set_before_read_hook(hook: impl FnOnce() + 'static) {
    BEFORE_READ.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(in crate::codex_hooks) fn set_after_load_hook(hook: impl FnOnce() + 'static) {
    AFTER_LOAD.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(in crate::codex_hooks) fn set_after_open_hook(hook: impl FnOnce() + 'static) {
    AFTER_OPEN.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(in crate::codex_hooks) fn set_after_operation_hook(hook: impl FnOnce() + 'static) {
    AFTER_OPERATION.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(super) fn run_before_read_hook() {
    run(&BEFORE_READ);
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(super) fn run_after_open_hook() {
    run(&AFTER_OPEN);
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(super) fn run_after_load_hook() {
    run(&AFTER_LOAD);
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(in crate::codex_hooks) fn run_after_operation_hook() {
    run(&AFTER_OPERATION);
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(super) fn run_before_persist_commit_hook(path: &Path) {
    let hook = persist_hooks()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(path);
    if let Some(hook) = hook {
        hook();
    }
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(super) fn run_before_persist_reproof_hook(path: &Path) {
    let hook = reproof_hooks()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(path);
    if let Some(hook) = hook {
        hook();
    }
}

#[cfg(any(test, feature = "runtime-component-tests"))]
fn run(slot: &'static std::thread::LocalKey<std::cell::RefCell<Option<Box<dyn FnOnce()>>>>) {
    if let Some(hook) = slot.with(|value| value.borrow_mut().take()) {
        hook();
    }
}

#[cfg(not(any(test, feature = "runtime-component-tests")))]
pub(super) fn run_before_read_hook() {}

#[cfg(not(any(test, feature = "runtime-component-tests")))]
pub(super) fn run_after_open_hook() {}

#[cfg(not(any(test, feature = "runtime-component-tests")))]
pub(super) fn run_after_load_hook() {}

#[cfg(not(any(test, feature = "runtime-component-tests")))]
pub(in crate::codex_hooks) fn run_after_operation_hook() {}

#[cfg(not(any(test, feature = "runtime-component-tests")))]
pub(super) fn run_before_persist_commit_hook(_path: &Path) {}

#[cfg(not(any(test, feature = "runtime-component-tests")))]
pub(super) fn run_before_persist_reproof_hook(_path: &Path) {}
