use anyhow::{Result, ensure};
use std::cell::RefCell;

const HOOK_ALREADY_INSTALLED_DETAIL: &str =
    "namespace-loss pre-finalize test hook is already installed";

type Hook = Box<dyn FnOnce() -> Result<()>>;

thread_local! {
    static HOOK: RefCell<Option<Hook>> = RefCell::new(None);
}

pub struct Guard;

pub fn install(hook: impl FnOnce() -> Result<()> + 'static) -> Result<Guard> {
    HOOK.with(|slot| {
        let mut slot = slot.borrow_mut();
        ensure!(slot.is_none(), HOOK_ALREADY_INSTALLED_DETAIL);
        *slot = Some(Box::new(hook));
        Ok(Guard)
    })
}

pub(super) fn invoke() -> Result<()> {
    let hook = HOOK.with(|slot| slot.borrow_mut().take());
    match hook {
        Some(hook) => hook(),
        None => Ok(()),
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        HOOK.with(|slot| {
            slot.borrow_mut().take();
        });
    }
}
