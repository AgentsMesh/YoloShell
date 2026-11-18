use super::private_file_names::unlink_isolation_name;
use super::*;
use std::ffi::CStr;

#[cfg(any(test, feature = "persistence-test-support"))]
#[path = "../../../test_support/private_file_slot_hooks.rs"]
mod test_hooks;
#[cfg(all(test, feature = "yolo-unit-tests"))]
pub(crate) use test_hooks::SlotHookGuard;

impl PrivateFileEntry {
    pub(super) fn move_named_identity(
        &self,
        source: &CStr,
        target: &CStr,
        expected: FileIdentity,
        max_bytes: u64,
    ) -> Result<()> {
        rename_exclusive_at(self.directory.descriptor(), source, target)
            .context("move private staging file")?;
        if let Err(error) = self.verify_named_identity(target, expected, max_bytes) {
            return self.rollback_named_move(source, target, error);
        }
        self.directory.sync()?;
        #[cfg(any(test, feature = "persistence-test-support"))]
        test_hooks::run_after_move_sync()?;
        if let Err(error) = self.verify_named_identity(target, expected, max_bytes) {
            return self.rollback_named_move(source, target, error);
        }
        Ok(())
    }

    pub fn unlink_named_identity<F>(
        &self,
        name: &CStr,
        expected: FileIdentity,
        max_bytes: u64,
        before_unlink: F,
    ) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        self.unlink_named_identity_with_durability(name, expected, max_bytes, before_unlink, true)
    }

    pub fn unlink_named_identity_deferred<F>(
        &self,
        name: &CStr,
        expected: FileIdentity,
        max_bytes: u64,
        before_unlink: F,
    ) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        self.unlink_named_identity_with_durability(name, expected, max_bytes, before_unlink, false)
    }

    fn unlink_named_identity_with_durability<F>(
        &self,
        name: &CStr,
        expected: FileIdentity,
        max_bytes: u64,
        before_unlink: F,
        sync_parent: bool,
    ) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        let isolation = unlink_isolation_name(name)?;
        let isolated = match self.open_named_optional(&isolation, max_bytes)? {
            Some(current) => {
                anyhow::ensure!(
                    entry_is_missing(&self.directory, name)?,
                    "private unlink source was replaced"
                );
                anyhow::ensure!(
                    current.identity.same_content_version(expected),
                    "private unlink isolation identity changed"
                );
                current.identity
            }
            None => {
                self.verify_named_identity(name, expected, max_bytes)?;
                before_unlink()?;
                self.verify_named_identity(name, expected, max_bytes)?;
                rename_exclusive_at(self.directory.descriptor(), name, &isolation)
                    .context("isolate private file identity")?;
                self.directory.sync()?;
                let current = self.open_named_required(&isolation, max_bytes)?;
                anyhow::ensure!(
                    current.identity.same_content_version(expected),
                    "private unlink isolation identity changed"
                );
                current.identity
            }
        };
        #[cfg(any(test, feature = "persistence-test-support"))]
        test_hooks::run_before_unlink()?;
        self.verify_named_identity(&isolation, isolated, max_bytes)?;
        #[cfg(any(test, feature = "persistence-test-support"))]
        test_hooks::run_before_unlink_syscall()?;
        let result = unsafe { libc::unlinkat(self.directory.descriptor(), isolation.as_ptr(), 0) };
        if result != 0 {
            return Err(std::io::Error::last_os_error()).context("unlink private file identity");
        }
        #[cfg(any(test, feature = "persistence-test-support"))]
        test_hooks::run_after_unlink_before_sync(&self.path)?;
        if sync_parent {
            self.directory.sync()?;
        }
        anyhow::ensure!(
            entry_is_missing(&self.directory, &isolation)?,
            "private unlink isolation was replaced after unlink"
        );
        anyhow::ensure!(
            entry_is_missing(&self.directory, name)?,
            "private unlink source was replaced"
        );
        self.directory.verify_path()
    }

    pub(super) fn unlink_isolation_matches(
        &self,
        name: &CStr,
        expected: FileIdentity,
        max_bytes: u64,
    ) -> Result<bool> {
        let isolation = unlink_isolation_name(name)?;
        let Some(current) = self.open_named_optional(&isolation, max_bytes)? else {
            return Ok(false);
        };
        anyhow::ensure!(
            current.identity.same_content_version(expected),
            "private unlink isolation identity changed"
        );
        Ok(true)
    }

    fn open_named_optional(
        &self,
        name: &CStr,
        max_bytes: u64,
    ) -> Result<Option<OpenedPrivateFile>> {
        match self.open_named_required(name, max_bytes) {
            Ok(opened) => Ok(Some(opened)),
            Err(error) if io_kind(&error) == Some(std::io::ErrorKind::NotFound) => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn open_named_required(&self, name: &CStr, max_bytes: u64) -> Result<OpenedPrivateFile> {
        open_named(
            self.directory.descriptor(),
            name,
            &self.path,
            read_flags(),
            max_bytes,
        )
    }

    fn rollback_named_move(
        &self,
        source: &CStr,
        target: &CStr,
        cause: anyhow::Error,
    ) -> Result<()> {
        let restored = rename_exclusive_at(self.directory.descriptor(), target, source);
        self.directory.sync()?;
        match restored {
            Ok(()) => Err(cause.context("private staging move was rolled back")),
            Err(error) => Err(cause.context(format!(
                "private staging replacement was retained at destination: {error}"
            ))),
        }
    }

    fn verify_named_identity(
        &self,
        name: &CStr,
        expected: FileIdentity,
        max_bytes: u64,
    ) -> Result<()> {
        let current = open_named(
            self.directory.descriptor(),
            name,
            &self.path,
            read_flags(),
            max_bytes,
        )?;
        anyhow::ensure!(
            current.identity.same_content_version(expected),
            "private staging file changed"
        );
        Ok(())
    }
}
