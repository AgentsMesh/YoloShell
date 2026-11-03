use super::*;
use std::io::Write;

const CONTENT_LIMIT: &str = "recoverable private file contents exceed limit";
const CREATE_CONTEXT: &str = "create recoverable private file";
const SYNC_CONTEXT: &str = "sync recoverable private file";
const WRITE_CONTEXT: &str = "write recoverable private file";
#[cfg(any(test, feature = "persistence-test-support"))]
const HOOK_OCCUPIED: &str = "recoverable create hook is occupied";

impl PrivateFileEntry {
    pub fn create_recoverable_bytes_exclusive(
        &self,
        contents: &[u8],
        max_bytes: u64,
    ) -> Result<FileIdentity> {
        anyhow::ensure!(u64::try_from(contents.len())? <= max_bytes, CONTENT_LIMIT);
        private_file_missing::run_before_create_hook();
        let flags = write_flags() | libc::O_CREAT | libc::O_EXCL;
        let mut file =
            open_at(self.directory.descriptor(), &self.name, flags).context(CREATE_CONTEXT)?;
        set_private_mode(&file, &self.path).context(CREATE_CONTEXT)?;
        run_after_create()?;
        file.write_all(contents).context(WRITE_CONTEXT)?;
        file.sync_all().context(SYNC_CONTEXT)?;
        let identity = FileIdentity::inspect_file_bounded(&file, &self.path, max_bytes)?;
        self.verify_current_unchanged(identity, max_bytes)?;
        self.directory.sync().context(SYNC_CONTEXT)?;
        self.verify_current_unchanged(identity, max_bytes)?;
        Ok(identity)
    }
}

#[cfg(any(test, feature = "persistence-test-support"))]
thread_local! {
    static AFTER_CREATE: std::cell::RefCell<Option<Box<dyn FnOnce() -> Result<()>>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(any(test, feature = "persistence-test-support"))]
pub(crate) struct RecoverableCreateHook;

#[cfg(any(test, feature = "persistence-test-support"))]
pub(crate) fn install_recoverable_create_hook(
    hook: impl FnOnce() -> Result<()> + 'static,
) -> RecoverableCreateHook {
    AFTER_CREATE.with(|slot| {
        let previous = slot.borrow_mut().replace(Box::new(hook));
        assert!(previous.is_none(), "{HOOK_OCCUPIED}");
    });
    RecoverableCreateHook
}

#[cfg(any(test, feature = "persistence-test-support"))]
impl Drop for RecoverableCreateHook {
    fn drop(&mut self) {
        AFTER_CREATE.with(|slot| *slot.borrow_mut() = None);
    }
}

#[cfg(any(test, feature = "persistence-test-support"))]
fn run_after_create() -> Result<()> {
    match AFTER_CREATE.with(|slot| slot.borrow_mut().take()) {
        Some(hook) => hook(),
        None => Ok(()),
    }
}

#[cfg(not(any(test, feature = "persistence-test-support")))]
fn run_after_create() -> Result<()> {
    Ok(())
}
