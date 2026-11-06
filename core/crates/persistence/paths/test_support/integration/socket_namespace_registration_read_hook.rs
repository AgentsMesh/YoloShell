use std::cell::RefCell;

thread_local! {
    static BEFORE_VERIFICATION: RefCell<Option<Box<dyn FnOnce()>>> = const { RefCell::new(None) };
}

pub(in super::super) fn install(hook: impl FnOnce() + 'static) {
    BEFORE_VERIFICATION.with(|slot| {
        assert!(slot.borrow().is_none());
        *slot.borrow_mut() = Some(Box::new(hook));
    });
}

pub(super) fn before_verification() {
    let hook = BEFORE_VERIFICATION.with(|slot| slot.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
    }
}
