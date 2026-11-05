use super::*;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const HOOK_ALREADY_INSTALLED: &str = "private file slot hook is already installed";

type ScopedHook = Box<dyn FnOnce() -> Result<()>>;
type CrossThreadHook = Box<dyn FnOnce() -> Result<()> + Send>;

fn unlink_hooks() -> &'static Mutex<std::collections::HashMap<PathBuf, CrossThreadHook>> {
    static HOOKS: OnceLock<Mutex<std::collections::HashMap<PathBuf, CrossThreadHook>>> =
        OnceLock::new();
    HOOKS.get_or_init(Default::default)
}

thread_local! {
    static AFTER_MOVE_SYNC: RefCell<Option<ScopedHook>> = RefCell::new(None);
    static BEFORE_UNLINK: RefCell<Option<ScopedHook>> = RefCell::new(None);
    static BEFORE_UNLINK_SYSCALL: RefCell<Option<ScopedHook>> = RefCell::new(None);
}

pub(crate) struct SlotHookGuard {
    unlink_path: Option<PathBuf>,
}

impl PrivateFileEntry {
    pub(crate) fn install_after_move_sync_hook(
        &self,
        hook: impl FnOnce() -> Result<()> + 'static,
    ) -> SlotHookGuard {
        install(&AFTER_MOVE_SYNC, hook)
    }

    pub(crate) fn install_before_unlink_hook(
        &self,
        hook: impl FnOnce() -> Result<()> + 'static,
    ) -> SlotHookGuard {
        install(&BEFORE_UNLINK, hook)
    }

    pub(crate) fn install_before_unlink_syscall_hook(
        &self,
        hook: impl FnOnce() -> Result<()> + 'static,
    ) -> SlotHookGuard {
        install(&BEFORE_UNLINK_SYSCALL, hook)
    }

    pub(crate) fn install_after_unlink_before_sync_hook(
        &self,
        hook: impl FnOnce() -> Result<()> + Send + 'static,
    ) -> SlotHookGuard {
        let path = self.path.clone();
        let previous = unlink_hooks()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(path.clone(), Box::new(hook));
        assert!(previous.is_none(), "{HOOK_ALREADY_INSTALLED}");
        SlotHookGuard {
            unlink_path: Some(path),
        }
    }
}

pub(super) fn run_after_move_sync() -> Result<()> {
    run(&AFTER_MOVE_SYNC)
}

pub(super) fn run_before_unlink() -> Result<()> {
    run(&BEFORE_UNLINK)
}

pub(super) fn run_before_unlink_syscall() -> Result<()> {
    run(&BEFORE_UNLINK_SYSCALL)
}

pub(super) fn run_after_unlink_before_sync(path: &Path) -> Result<()> {
    let hook = unlink_hooks()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(path);
    match hook {
        Some(hook) => hook(),
        None => Ok(()),
    }
}

fn install(
    slot: &'static std::thread::LocalKey<RefCell<Option<ScopedHook>>>,
    hook: impl FnOnce() -> Result<()> + 'static,
) -> SlotHookGuard {
    slot.with(|slot| {
        let previous = slot.borrow_mut().replace(Box::new(hook));
        assert!(previous.is_none(), "{HOOK_ALREADY_INSTALLED}");
    });
    SlotHookGuard { unlink_path: None }
}

fn run(slot: &'static std::thread::LocalKey<RefCell<Option<ScopedHook>>>) -> Result<()> {
    match slot.with(|slot| slot.borrow_mut().take()) {
        Some(hook) => hook(),
        None => Ok(()),
    }
}

impl Drop for SlotHookGuard {
    fn drop(&mut self) {
        AFTER_MOVE_SYNC.with(|slot| *slot.borrow_mut() = None);
        BEFORE_UNLINK.with(|slot| *slot.borrow_mut() = None);
        BEFORE_UNLINK_SYSCALL.with(|slot| *slot.borrow_mut() = None);
        if let Some(path) = self.unlink_path.take() {
            unlink_hooks()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&path);
        }
    }
}
