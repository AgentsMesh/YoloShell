#[cfg(test)]
type PreSignalHook = Box<dyn FnOnce()>;

#[cfg(test)]
thread_local! {
    static PRE_SIGNAL_HOOK: std::cell::RefCell<Option<PreSignalHook>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(super) struct TestSignalHookGuard;

#[cfg(test)]
pub(super) fn install_pre_signal_hook(hook: impl FnOnce() + 'static) -> TestSignalHookGuard {
    PRE_SIGNAL_HOOK.with(|slot| {
        let previous = slot.borrow_mut().replace(Box::new(hook));
        assert!(previous.is_none(), "pre-signal hook is already installed");
    });
    TestSignalHookGuard
}

#[cfg(test)]
pub(super) fn run_pre_signal_hook() {
    if let Some(hook) = PRE_SIGNAL_HOOK.with(|slot| slot.borrow_mut().take()) {
        hook();
    }
}

#[cfg(not(test))]
pub(super) fn run_pre_signal_hook() {}

#[cfg(test)]
impl Drop for TestSignalHookGuard {
    fn drop(&mut self) {
        PRE_SIGNAL_HOOK.with(|slot| *slot.borrow_mut() = None);
    }
}
