use super::PrivateDirectory;
use super::private_file_io::entry_stat;
use anyhow::Result;
use std::ffi::CStr;

pub(super) fn entry_is_missing(parent: &PrivateDirectory, name: &CStr) -> Result<bool> {
    entry_is_missing_with(parent, name, || Ok(()))
}

pub(super) fn retained_entry_is_missing(parent: &PrivateDirectory, name: &CStr) -> Result<bool> {
    match entry_stat(parent.descriptor(), name) {
        Ok(_) => Ok(false),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            entry_stat(parent.descriptor(), name)
                .map(|_| false)
                .or_else(|error| match error.kind() {
                    std::io::ErrorKind::NotFound => Ok(true),
                    _ => Err(error),
                })
                .map_err(Into::into)
        }
        Err(error) => Err(error.into()),
    }
}

fn entry_is_missing_with<F>(parent: &PrivateDirectory, name: &CStr, hook: F) -> Result<bool>
where
    F: FnOnce() -> Result<()>,
{
    match entry_stat(parent.descriptor(), name) {
        Ok(stat) => {
            anyhow::ensure!(
                stat.st_mode & libc::S_IFMT == libc::S_IFREG,
                "private entry is not regular"
            );
            Ok(false)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            hook()?;
            parent.verify_descriptor_path()?;
            let missing = entry_stat(parent.descriptor(), name)
                .map(|_| false)
                .or_else(|error| match error.kind() {
                    std::io::ErrorKind::NotFound => Ok(true),
                    _ => Err(error),
                })?;
            parent.verify_descriptor_path()?;
            Ok(missing)
        }
        Err(error) => Err(error.into()),
    }
}

#[cfg(any(test, feature = "persistence-test-support"))]
thread_local! {
    static BEFORE_CREATE_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(any(test, feature = "persistence-test-support"))]
pub(crate) fn set_before_create_hook(hook: impl FnOnce() + 'static) {
    BEFORE_CREATE_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(any(test, feature = "persistence-test-support"))]
pub(super) fn run_before_create_hook() {
    let hook = BEFORE_CREATE_HOOK.with(|slot| slot.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
    }
}

#[cfg(not(any(test, feature = "persistence-test-support")))]
pub(super) fn run_before_create_hook() {}

#[cfg(test)]
pub(crate) fn entry_is_missing_with_hook<F>(
    parent: &PrivateDirectory,
    name: &CStr,
    hook: F,
) -> Result<bool>
where
    F: FnOnce() -> Result<()>,
{
    entry_is_missing_with(parent, name, hook)
}
