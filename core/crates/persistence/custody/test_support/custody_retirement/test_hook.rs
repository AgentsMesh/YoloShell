use anyhow::Result;
use std::cell::RefCell;

const HOOK_OCCUPIED: &str = "custody retirement durability test hook is occupied";
type ReceiptHook = Box<dyn FnOnce() -> Result<()>>;

thread_local! {
    static AFTER_RECEIPT_PARENT_SYNC: RefCell<Option<ReceiptHook>> = const { RefCell::new(None) };
}

pub(super) struct HookGuard;

pub(super) fn install_after_receipt_parent_sync(
    hook: impl FnOnce() -> Result<()> + 'static,
) -> HookGuard {
    AFTER_RECEIPT_PARENT_SYNC.with(|slot| {
        let previous = slot.borrow_mut().replace(Box::new(hook));
        assert!(previous.is_none(), "{HOOK_OCCUPIED}");
    });
    HookGuard
}

pub(super) fn run_after_receipt_parent_sync() -> Result<()> {
    match AFTER_RECEIPT_PARENT_SYNC.with(|slot| slot.borrow_mut().take()) {
        Some(hook) => hook(),
        None => Ok(()),
    }
}

impl Drop for HookGuard {
    fn drop(&mut self) {
        AFTER_RECEIPT_PARENT_SYNC.with(|slot| *slot.borrow_mut() = None);
    }
}
