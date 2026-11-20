use anyhow::Result;
use std::cell::RefCell;

const HOOK_ALREADY_INSTALLED: &str = "probe cleanup rename hook is already installed";

type RenameHook = Box<dyn FnOnce() -> Result<()>>;

thread_local! {
    static BEFORE_RENAME: RefCell<Option<RenameHook>> = RefCell::new(None);
}

pub(crate) struct HookGuard;

pub(crate) fn install_before_rename(hook: impl FnOnce() -> Result<()> + 'static) -> HookGuard {
    BEFORE_RENAME.with(|slot| {
        let previous = slot.borrow_mut().replace(Box::new(hook));
        assert!(previous.is_none(), "{HOOK_ALREADY_INSTALLED}");
    });
    HookGuard
}

pub(super) fn run_before_rename() -> Result<()> {
    let hook = BEFORE_RENAME.with(|slot| slot.borrow_mut().take());
    match hook {
        Some(hook) => hook(),
        None => Ok(()),
    }
}

impl Drop for HookGuard {
    fn drop(&mut self) {
        BEFORE_RENAME.with(|slot| *slot.borrow_mut() = None);
    }
}
