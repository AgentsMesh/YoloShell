use super::private_file_io::{open_named, read_flags, rename_exclusive_at};
use super::private_file_missing::retained_entry_is_missing;
use super::private_file_retained::{OwnedRetainedName, RetainedCleanup, RetainedPrivateFile};
use super::*;

impl RetainedPrivateFile {
    pub fn cleanup(&mut self) -> Result<()> {
        if matches!(self.cleanup, RetainedCleanup::PreserveSyncPending) {
            return self.sync_cleanup_parent();
        }
        let destination = self.name.clone();
        if let RetainedCleanup::Named(names) = &mut self.cleanup {
            while let Some(owned) = names.last() {
                recycle_owned_name(&self.directory, &self.path, owned, &destination)?;
                names.pop();
            }
            self.cleanup = RetainedCleanup::ParentSyncPending;
        }
        if !matches!(self.cleanup, RetainedCleanup::ParentSyncPending) {
            return Ok(());
        }
        self.sync_cleanup_parent()
    }

    pub(super) fn replace_cleanup(&mut self, name: CString, identity: FileIdentity) {
        self.cleanup = RetainedCleanup::Named(vec![OwnedRetainedName { identity, name }]);
    }

    pub fn preserve(&mut self) {
        self.cleanup = RetainedCleanup::PreserveSyncPending;
        let _ = self.cleanup();
    }

    fn sync_cleanup_parent(&mut self) -> Result<()> {
        run_cleanup_sync_test_hook()?;
        self.directory.sync()?;
        self.cleanup = RetainedCleanup::Complete;
        Ok(())
    }
}

impl Drop for RetainedPrivateFile {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

fn recycle_owned_name(
    directory: &PrivateDirectory,
    path: &Path,
    owned: &OwnedRetainedName,
    destination: &CString,
) -> Result<()> {
    let current = open_named(
        directory.descriptor(),
        &owned.name,
        path,
        read_flags(),
        owned.identity.length(),
    )?;
    anyhow::ensure!(
        current.identity.same_content_version(owned.identity),
        "retained recycle source changed"
    );
    anyhow::ensure!(
        retained_entry_is_missing(directory, destination)?,
        "retained recycle destination is occupied"
    );
    run_cleanup_recycle_test_hook()?;
    rename_exclusive_at(directory.descriptor(), &owned.name, destination)
        .context("recycle retained private file")?;
    let installed = open_named(
        directory.descriptor(),
        destination,
        path,
        read_flags(),
        owned.identity.length(),
    );
    if installed
        .as_ref()
        .is_ok_and(|opened| opened.identity.same_content_version(owned.identity))
    {
        return Ok(());
    }
    let rollback = rename_exclusive_at(directory.descriptor(), destination, &owned.name);
    let _ = directory.sync();
    match rollback {
        Ok(()) => anyhow::bail!("invalid retained recycle was rolled back"),
        Err(error) => anyhow::bail!("invalid retained recycle was preserved: {error}"),
    }
}

#[cfg(any(test, feature = "persistence-test-support"))]
type FallibleHook = Option<Box<dyn FnOnce() -> Result<()>>>;

#[cfg(any(test, feature = "persistence-test-support"))]
thread_local! {
    static CLEANUP_SYNC_HOOK: std::cell::RefCell<FallibleHook> =
        const { std::cell::RefCell::new(None) };
    static CLEANUP_RECYCLE_HOOK: std::cell::RefCell<FallibleHook> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(any(test, feature = "persistence-test-support"))]
pub(crate) fn set_retained_cleanup_sync_hook<F: FnOnce() -> Result<()> + 'static>(hook: F) {
    CLEANUP_SYNC_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(any(test, feature = "persistence-test-support"))]
pub(crate) fn set_retained_cleanup_recycle_hook<F: FnOnce() -> Result<()> + 'static>(hook: F) {
    CLEANUP_RECYCLE_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

fn run_cleanup_sync_test_hook() -> Result<()> {
    #[cfg(any(test, feature = "persistence-test-support"))]
    return CLEANUP_SYNC_HOOK.with(run_hook);
    #[cfg(not(any(test, feature = "persistence-test-support")))]
    Ok(())
}

fn run_cleanup_recycle_test_hook() -> Result<()> {
    #[cfg(any(test, feature = "persistence-test-support"))]
    return CLEANUP_RECYCLE_HOOK.with(run_hook);
    #[cfg(not(any(test, feature = "persistence-test-support")))]
    Ok(())
}

#[cfg(any(test, feature = "persistence-test-support"))]
fn run_hook(slot: &std::cell::RefCell<FallibleHook>) -> Result<()> {
    match slot.borrow_mut().take() {
        Some(hook) => hook(),
        None => Ok(()),
    }
}
