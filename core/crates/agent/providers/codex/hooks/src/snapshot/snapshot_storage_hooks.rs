#[cfg(any(test, feature = "runtime-component-tests"))]
thread_local! {
    static AFTER_OPEN: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
    static AFTER_PERSIST: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
    static BEFORE_PERSIST_COMMIT: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
    static BEFORE_READ: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(in crate::codex_hooks) fn set_after_open_hook(hook: impl FnOnce() + 'static) {
    AFTER_OPEN.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(in crate::codex_hooks) fn set_after_persist_hook(hook: impl FnOnce() + 'static) {
    AFTER_PERSIST.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(in crate::codex_hooks) fn set_before_persist_commit_hook(hook: impl FnOnce() + 'static) {
    BEFORE_PERSIST_COMMIT.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(in crate::codex_hooks) fn set_before_read_hook(hook: impl FnOnce() + 'static) {
    BEFORE_READ.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(super) fn run_after_open() {
    run(&AFTER_OPEN);
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(super) fn run_after_persist() {
    run(&AFTER_PERSIST);
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(super) fn run_before_persist_commit() {
    run(&BEFORE_PERSIST_COMMIT);
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(super) fn run_before_read() {
    run(&BEFORE_READ);
}

#[cfg(any(test, feature = "runtime-component-tests"))]
fn run(slot: &'static std::thread::LocalKey<std::cell::RefCell<Option<Box<dyn FnOnce()>>>>) {
    if let Some(hook) = slot.with(|value| value.borrow_mut().take()) {
        hook();
    }
}

#[cfg(not(any(test, feature = "runtime-component-tests")))]
pub(super) fn run_after_open() {}

#[cfg(not(any(test, feature = "runtime-component-tests")))]
pub(super) fn run_after_persist() {}

#[cfg(not(any(test, feature = "runtime-component-tests")))]
pub(super) fn run_before_persist_commit() {}

#[cfg(not(any(test, feature = "runtime-component-tests")))]
pub(super) fn run_before_read() {}
