use anyhow::Result;

#[cfg(any(test, feature = "runtime-component-tests"))]
thread_local! {
    static AFTER_ACQUIRE: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
    static BEFORE_AUTHORITY_CAPTURE: std::cell::RefCell<
        Option<Box<dyn FnOnce() -> Result<()>>>,
    > = const { std::cell::RefCell::new(None) };
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(in crate::codex_hooks) fn set_after_acquire(hook: impl FnOnce() + 'static) {
    AFTER_ACQUIRE.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(in crate::codex_hooks) fn set_before_authority_capture(
    hook: impl FnOnce() -> Result<()> + 'static,
) {
    BEFORE_AUTHORITY_CAPTURE.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(super) fn run_after_acquire() {
    if let Some(hook) = AFTER_ACQUIRE.with(|slot| slot.borrow_mut().take()) {
        hook();
    }
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(super) fn run_before_authority_capture() -> Result<()> {
    match BEFORE_AUTHORITY_CAPTURE.with(|slot| slot.borrow_mut().take()) {
        Some(hook) => hook(),
        None => Ok(()),
    }
}

#[cfg(not(any(test, feature = "runtime-component-tests")))]
pub(super) fn run_after_acquire() {}

#[cfg(not(any(test, feature = "runtime-component-tests")))]
pub(super) fn run_before_authority_capture() -> Result<()> {
    Ok(())
}
