use anyhow::Result;
use std::ffi::CString;
use std::path::Path;

#[cfg(test)]
use std::cell::RefCell;

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FaultPoint {
    CreationAfterMkdir,
    CreationBeforeOpen,
    CreationAfterOpen,
    AdoptBeforeVerify,
    RetireBeforeRename,
    RetireAfterRename,
    RetireBeforeOpen,
    RetireBeforeRootUnlink,
}

#[cfg(test)]
type FaultHook = Box<dyn FnMut(FaultPoint, &Path) -> Result<()>>;
#[cfg(test)]
type NameHook = Box<dyn FnMut(&str) -> Result<Option<CString>>>;

#[cfg(test)]
thread_local! {
    static FAULT_HOOK: RefCell<Option<FaultHook>> = RefCell::new(None);
    static NAME_HOOK: RefCell<Option<NameHook>> = RefCell::new(None);
}

#[cfg(test)]
pub(crate) struct FaultHookGuard;
#[cfg(test)]
pub(crate) struct NameHookGuard;

#[cfg(test)]
impl Drop for FaultHookGuard {
    fn drop(&mut self) {
        FAULT_HOOK.with(|slot| *slot.borrow_mut() = None);
    }
}

#[cfg(test)]
impl Drop for NameHookGuard {
    fn drop(&mut self) {
        NAME_HOOK.with(|slot| *slot.borrow_mut() = None);
    }
}

#[cfg(test)]
pub(crate) fn set_fault_hook(hook: FaultHook) -> FaultHookGuard {
    FAULT_HOOK.with(|slot| *slot.borrow_mut() = Some(hook));
    FaultHookGuard
}

#[cfg(test)]
pub(crate) fn set_name_hook(hook: NameHook) -> NameHookGuard {
    NAME_HOOK.with(|slot| *slot.borrow_mut() = Some(hook));
    NameHookGuard
}

#[cfg(test)]
pub(super) fn generated_name(prefix: &str) -> Result<Option<CString>> {
    NAME_HOOK.with(|slot| match slot.borrow_mut().as_mut() {
        Some(hook) => hook(prefix),
        None => Ok(None),
    })
}

#[cfg(not(test))]
pub(super) fn generated_name(_prefix: &str) -> Result<Option<CString>> {
    Ok(None)
}

#[cfg(test)]
fn invoke(point: FaultPoint, path: &Path) -> Result<()> {
    FAULT_HOOK.with(|slot| match slot.borrow_mut().as_mut() {
        Some(hook) => hook(point, path),
        None => Ok(()),
    })
}

#[cfg(not(test))]
fn invoke(_point: (), _path: &Path) -> Result<()> {
    Ok(())
}

macro_rules! hook {
    ($name:ident, $point:ident) => {
        pub(super) fn $name(path: &Path) -> Result<()> {
            #[cfg(test)]
            return invoke(FaultPoint::$point, path);
            #[cfg(not(test))]
            invoke((), path)
        }
    };
}

hook!(creation_after_mkdir, CreationAfterMkdir);
hook!(creation_before_open, CreationBeforeOpen);
hook!(creation_after_open, CreationAfterOpen);
#[cfg(any(test, feature = "adversarial-test-hooks"))]
hook!(adopt_before_verify, AdoptBeforeVerify);
hook!(retire_before_rename, RetireBeforeRename);
hook!(retire_after_rename, RetireAfterRename);
hook!(retire_before_open, RetireBeforeOpen);
hook!(retire_before_root_unlink, RetireBeforeRootUnlink);
