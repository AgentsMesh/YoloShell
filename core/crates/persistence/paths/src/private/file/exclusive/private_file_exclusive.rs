use super::private_file_names::active_name;
use super::*;
use crate::paths::constants::{ATOMIC_FILE_PREFIX, ATOMIC_FILE_SUFFIX};
use std::io::Write;

const CLEANUP_CONTEXT: &str = "clean exclusive private file staging";
const CONTENT_LIMIT: &str = "private file contents exceed limit";
#[cfg(test)]
const HOOK_OCCUPIED: &str = "exclusive publish hook is occupied";
const TARGET_CONFLICT: &str = "private file already exists";

#[cfg(test)]
thread_local! {
    static AFTER_PUBLISH: std::cell::RefCell<Option<Box<dyn FnOnce() -> Result<()>>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(crate) struct ExclusivePublishHook;

#[cfg(test)]
pub(crate) fn install_exclusive_publish_hook(
    hook: impl FnOnce() -> Result<()> + 'static,
) -> ExclusivePublishHook {
    AFTER_PUBLISH.with(|slot| {
        let previous = slot.borrow_mut().replace(Box::new(hook));
        assert!(previous.is_none(), "{HOOK_OCCUPIED}");
    });
    ExclusivePublishHook
}

#[cfg(test)]
impl Drop for ExclusivePublishHook {
    fn drop(&mut self) {
        AFTER_PUBLISH.with(|slot| *slot.borrow_mut() = None);
    }
}

#[cfg(test)]
fn run_after_publish() -> Result<()> {
    match AFTER_PUBLISH.with(|slot| slot.borrow_mut().take()) {
        Some(hook) => hook(),
        None => Ok(()),
    }
}

impl PrivateFileEntry {
    pub fn exclusive_create_staging_entry(&self) -> Result<Self> {
        let name = active_name(&self.name, ATOMIC_FILE_PREFIX, ATOMIC_FILE_SUFFIX)?;
        self.sibling_named(&name)
    }

    pub fn create_bytes_exclusive(&self, contents: &[u8], max_bytes: u64) -> Result<FileIdentity> {
        self.create_bytes_exclusive_initialized(contents, max_bytes, |_| Ok(()))
    }

    pub fn create_bytes_exclusive_initialized<I>(
        &self,
        contents: &[u8],
        max_bytes: u64,
        initializer: I,
    ) -> Result<FileIdentity>
    where
        I: FnMut(&std::fs::File) -> Result<()>,
    {
        self.create_bytes_exclusive_initialized_with_fence(contents, max_bytes, initializer, || {
            Ok(())
        })
    }

    pub fn create_bytes_exclusive_with_fence<F>(
        &self,
        contents: &[u8],
        max_bytes: u64,
        commit_fence: F,
    ) -> Result<FileIdentity>
    where
        F: FnMut() -> Result<()>,
    {
        self.create_bytes_exclusive_initialized_with_fence(
            contents,
            max_bytes,
            |_| Ok(()),
            commit_fence,
        )
    }

    fn create_bytes_exclusive_initialized_with_fence<I, F>(
        &self,
        contents: &[u8],
        max_bytes: u64,
        mut initializer: I,
        mut commit_fence: F,
    ) -> Result<FileIdentity>
    where
        I: FnMut(&std::fs::File) -> Result<()>,
        F: FnMut() -> Result<()>,
    {
        anyhow::ensure!(u64::try_from(contents.len())? <= max_bytes, CONTENT_LIMIT);
        let mut temporary = self.temporary(ATOMIC_FILE_PREFIX, ATOMIC_FILE_SUFFIX)?;
        let result = self.install_exclusive_bytes(
            &mut temporary,
            contents,
            max_bytes,
            &mut initializer,
            &mut commit_fence,
        );
        match result {
            Ok(identity) => Ok(identity),
            Err(error) => match self.rollback_exclusive_create(&temporary, max_bytes) {
                Ok(()) => Err(error),
                Err(cleanup) => Err(error.context(format!("{CLEANUP_CONTEXT}: {cleanup:#}"))),
            },
        }
    }

    fn install_exclusive_bytes(
        &self,
        temporary: &mut TemporaryPrivateFile,
        contents: &[u8],
        max_bytes: u64,
        initializer: &mut impl FnMut(&std::fs::File) -> Result<()>,
        commit_fence: &mut impl FnMut() -> Result<()>,
    ) -> Result<FileIdentity> {
        anyhow::ensure!(temporary.previous.is_none(), TARGET_CONFLICT);
        temporary.file.write_all(contents)?;
        initializer(&temporary.file)?;
        temporary.sync_and_inspect(&self.path, max_bytes)?;
        self.verify_expected(None, max_bytes)?;
        self.rename_temporary_with_fences(
            temporary,
            max_bytes,
            || self.verify_expected(None, max_bytes),
            &mut *commit_fence,
            || {},
        )?;
        commit_fence()?;
        #[cfg(test)]
        run_after_publish()?;
        let installed = self.open_read(max_bytes)?;
        anyhow::ensure!(
            installed.identity.same_content_version(temporary.identity),
            TARGET_CONFLICT
        );
        Ok(installed.identity)
    }

    fn rollback_exclusive_create(
        &self,
        temporary: &TemporaryPrivateFile,
        max_bytes: u64,
    ) -> Result<()> {
        self.discard_owned_name(
            &temporary.name,
            &temporary.file,
            temporary.identity,
            max_bytes,
        )?;
        let Some(current) = self.inspect(max_bytes)? else {
            return Ok(());
        };
        if current.same_content_version(temporary.identity) {
            anyhow::ensure!(
                self.discard_owned_name(
                    &self.name,
                    &temporary.file,
                    temporary.identity,
                    max_bytes,
                )?,
                TARGET_CONFLICT
            );
        }
        Ok(())
    }
}
