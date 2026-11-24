use super::secure_vfs_alias_cleanup::CleanupIoPoint;
use std::cell::RefCell;
use std::path::Path;

type PathHook = Box<dyn FnOnce(&Path)>;

thread_local! {
    static RETAIN_HOOK: RefCell<Option<PathHook>> = RefCell::new(None);
    static REMOVE_HOOK: RefCell<Option<PathHook>> = RefCell::new(None);
    static CLEANUP_IO_HOOK: RefCell<Option<CleanupIoPoint>> = const { RefCell::new(None) };
}

pub(super) fn set_retain_hook(hook: impl FnOnce(&Path) + 'static) {
    RETAIN_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

pub(super) fn run_retain_hook(path: &Path) {
    let hook = RETAIN_HOOK.with(|slot| slot.borrow_mut().take());
    if let Some(hook) = hook {
        hook(path);
    }
}

pub(super) fn set_remove_hook(hook: impl FnOnce(&Path) + 'static) {
    REMOVE_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

pub(super) fn run_remove_hook(path: &Path) {
    let hook = REMOVE_HOOK.with(|slot| slot.borrow_mut().take());
    if let Some(hook) = hook {
        hook(path);
    }
}

pub(super) fn set_cleanup_io_hook(point: CleanupIoPoint) {
    CLEANUP_IO_HOOK.with(|slot| *slot.borrow_mut() = Some(point));
}

pub(super) fn run_cleanup_io_hook(point: CleanupIoPoint) -> anyhow::Result<()> {
    let injected = CLEANUP_IO_HOOK.with(|slot| {
        let mut stored = slot.borrow_mut();
        let matches = *stored == Some(point);
        if matches {
            *stored = None;
        }
        matches
    });
    if injected {
        return Err(std::io::Error::from_raw_os_error(libc::EIO).into());
    }
    Ok(())
}
