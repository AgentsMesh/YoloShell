use super::*;
use std::cell::{Cell, RefCell};

const RETAINED_PARENT_SYNC_FAULT: &str = "injected retained cursor parent sync failure";
const RETAINED_POST_INSTALL_FAULT: &str = "injected retained post-install failure";
const RETAINED_CONFIRM_FAULT: &str = "injected retained confirmation failure";
type FallibleHookSlot = Option<Box<dyn FnOnce() -> anyhow::Result<()>>>;

thread_local! {
    static PARENT_SYNC_HOOK: RefCell<FallibleHookSlot> = const { RefCell::new(None) };
    static STAGING_HOOK: RefCell<FallibleHookSlot> = const { RefCell::new(None) };
    static POST_INSTALL_HOOK: RefCell<FallibleHookSlot> = const { RefCell::new(None) };
    static CONFIRM_HOOK: RefCell<FallibleHookSlot> = const { RefCell::new(None) };
    static CONFIRM_CURSOR_OVERRIDE: Cell<Option<u64>> = const { Cell::new(None) };
    static CONFIRM_SEQUENCE_OVERRIDE: Cell<Option<u64>> = const { Cell::new(None) };
}

pub(super) fn set_confirm_cursor_override(committed_bytes: u64) {
    CONFIRM_CURSOR_OVERRIDE.with(|slot| slot.set(Some(committed_bytes)));
}

pub(super) fn take_confirm_cursor_override() -> Option<u64> {
    CONFIRM_CURSOR_OVERRIDE.with(Cell::take)
}

pub(super) fn set_confirm_sequence_override(sequence: u64) {
    CONFIRM_SEQUENCE_OVERRIDE.with(|slot| slot.set(Some(sequence)));
}

pub(super) fn take_confirm_sequence_override() -> Option<u64> {
    CONFIRM_SEQUENCE_OVERRIDE.with(Cell::take)
}

pub(super) fn inject_parent_sync_failure() {
    set_parent_sync_hook(|| Err(std::io::Error::other(RETAINED_PARENT_SYNC_FAULT).into()));
}

pub(super) fn inject_post_install_failure() {
    set_post_install_hook(|| Err(std::io::Error::other(RETAINED_POST_INSTALL_FAULT).into()));
}

pub(super) fn inject_confirm_failure() {
    CONFIRM_HOOK.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(|| {
            Err(std::io::Error::other(RETAINED_CONFIRM_FAULT).into())
        }));
    });
}

pub(super) fn set_parent_sync_hook<F: FnOnce() -> anyhow::Result<()> + 'static>(hook: F) {
    PARENT_SYNC_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

pub(super) fn run_parent_sync_hook() -> Result<(), JournalError> {
    run(&PARENT_SYNC_HOOK)
}

pub(super) fn set_staging_hook<F: FnOnce() -> anyhow::Result<()> + 'static>(hook: F) {
    STAGING_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

pub(super) fn clear_staging_hook() {
    STAGING_HOOK.with(|slot| *slot.borrow_mut() = None);
}

pub(super) fn run_staging_hook() -> Result<(), JournalError> {
    run(&STAGING_HOOK)
}

pub(super) fn set_post_install_hook<F: FnOnce() -> anyhow::Result<()> + 'static>(hook: F) {
    POST_INSTALL_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

pub(super) fn run_post_install_hook() -> Result<(), JournalError> {
    run(&POST_INSTALL_HOOK)
}

pub(super) fn run_confirm_hook() -> Result<(), JournalError> {
    run(&CONFIRM_HOOK)
}

fn run(key: &'static std::thread::LocalKey<RefCell<FallibleHookSlot>>) -> Result<(), JournalError> {
    key.with(|slot| match slot.borrow_mut().take() {
        Some(hook) => hook().map_err(JournalError::from),
        None => Ok(()),
    })
}
