use std::cell::RefCell;

const HOOK_ALREADY_INSTALLED: &str = "macOS exec observer hook is already installed";

thread_local! {
    static AFTER_EVENT_HOOK: RefCell<Option<Box<dyn FnOnce()>>> = const { RefCell::new(None) };
}

pub struct AfterEventHookGuard;

pub(super) fn install_after_event_hook(hook: impl FnOnce() + 'static) -> AfterEventHookGuard {
    AFTER_EVENT_HOOK.with(|slot| {
        assert!(
            slot.borrow_mut().replace(Box::new(hook)).is_none(),
            "{HOOK_ALREADY_INSTALLED}"
        );
    });
    AfterEventHookGuard
}

pub(super) fn run_after_event_hook() {
    AFTER_EVENT_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook();
        }
    });
}

impl Drop for AfterEventHookGuard {
    fn drop(&mut self) {
        AFTER_EVENT_HOOK.with(|slot| slot.borrow_mut().take());
    }
}
