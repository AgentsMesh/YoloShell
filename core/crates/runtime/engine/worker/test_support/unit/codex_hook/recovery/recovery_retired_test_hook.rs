use std::cell::RefCell;

thread_local! {
    static BEFORE_FINAL_VALIDATION: RefCell<Option<Box<dyn FnOnce()>>> =
        const { RefCell::new(None) };
}

pub(super) fn set(hook: impl FnOnce() + 'static) {
    BEFORE_FINAL_VALIDATION.with(|slot| {
        let previous = slot.replace(Some(Box::new(hook)));
        assert!(previous.is_none());
    });
}

pub(super) fn run() {
    BEFORE_FINAL_VALIDATION.with(|slot| {
        if let Some(hook) = slot.take() {
            hook();
        }
    });
}
