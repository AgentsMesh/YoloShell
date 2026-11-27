use anyhow::Result;
use std::path::Path;

type AfterLockCallback = Option<Box<dyn FnOnce(&Path) -> Result<()>>>;

thread_local! {
    static AFTER_LOCK_HOOK: std::cell::RefCell<AfterLockCallback> = const {
        std::cell::RefCell::new(None)
    };
}

pub(super) fn set<F>(hook: F)
where
    F: FnOnce(&Path) -> Result<()> + 'static,
{
    AFTER_LOCK_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

pub(super) fn run(path: &Path) -> Result<()> {
    AFTER_LOCK_HOOK.with(|slot| match slot.borrow_mut().take() {
        Some(hook) => hook(path),
        None => Ok(()),
    })
}
