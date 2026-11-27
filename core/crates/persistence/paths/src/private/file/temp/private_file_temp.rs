use super::private_file_names::active_name;
use super::*;
use crate::paths::constants::{
    PRIVATE_FILE_TEMPORARY_ATTEMPTS, PRIVATE_FILE_TEMPORARY_RETRY_ERROR,
};

enum OptionalTarget {
    Ready(
        Option<(
            OpenedPrivateFile,
            Option<crate::paths::process_file_lock::ProcessFileLock>,
        )>,
    ),
    Retry,
}

impl PrivateFileEntry {
    pub fn temporary(&self, prefix: &str, suffix: &str) -> Result<TemporaryPrivateFile> {
        self.temporary_with(prefix, suffix, || Ok(()), || Ok(()), || Ok(()))
    }

    pub(crate) fn temporary_single_attempt(
        &self,
        prefix: &str,
        suffix: &str,
    ) -> Result<TemporaryPrivateFile> {
        let mut no_target_hook = || Ok(());
        let mut no_stage_hook = || Ok(());
        let mut no_truncate_hook = || Ok(());
        self.temporary_once_with_policy(
            prefix,
            suffix,
            &mut no_target_hook,
            &mut no_stage_hook,
            &mut no_truncate_hook,
            TargetLockPolicy::Acquire,
        )?
        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::WouldBlock))
        .context(PRIVATE_FILE_TEMPORARY_RETRY_ERROR)
    }

    pub(in crate::paths) fn temporary_with_target_lock_authority(
        &self,
        prefix: &str,
        suffix: &str,
        authority: &TargetLockAuthority<'_>,
    ) -> Result<TemporaryPrivateFile> {
        self.temporary_with_policy(
            prefix,
            suffix,
            || Ok(()),
            || Ok(()),
            || Ok(()),
            TargetLockPolicy::HeldByCaller(authority),
        )
    }

    pub(in crate::paths) fn temporary_with_serialized_target(
        &self,
        prefix: &str,
        suffix: &str,
        authority: &ParentLockAuthority<'_>,
    ) -> Result<TemporaryPrivateFile> {
        self.temporary_with_policy(
            prefix,
            suffix,
            || Ok(()),
            || Ok(()),
            || Ok(()),
            TargetLockPolicy::SerializedByCaller(authority),
        )
    }

    pub(super) fn temporary_with<F, G, H>(
        &self,
        prefix: &str,
        suffix: &str,
        after_target_open: F,
        after_stage_open: G,
        before_truncate: H,
    ) -> Result<TemporaryPrivateFile>
    where
        F: FnMut() -> Result<()>,
        G: FnMut() -> Result<()>,
        H: FnMut() -> Result<()>,
    {
        self.temporary_with_policy(
            prefix,
            suffix,
            after_target_open,
            after_stage_open,
            before_truncate,
            TargetLockPolicy::Acquire,
        )
    }

    pub fn remove_temporary(&self, temporary: &TemporaryPrivateFile) -> Result<()> {
        let current = open_named(
            self.directory.descriptor(),
            &temporary.name,
            &self.path,
            read_flags(),
            u64::MAX,
        );
        match current {
            Ok(opened) if opened.identity.same_object(temporary.identity) => {
                anyhow::ensure!(
                    retained_entry_is_missing(&self.directory, &temporary.slot)?,
                    "private staging slot is already occupied"
                );
                temporary
                    .file
                    .set_len(0)
                    .context("truncate failed private staging file")?;
                temporary
                    .file
                    .sync_all()
                    .context("sync failed private staging file")?;
                let identity =
                    FileIdentity::inspect_file_bounded(&temporary.file, &self.path, u64::MAX)?;
                self.move_named_identity(&temporary.name, &temporary.slot, identity, u64::MAX)
            }
            Err(error) if io_kind(&error) == Some(std::io::ErrorKind::NotFound) => Ok(()),
            Ok(_) => anyhow::bail!("temporary private file identity changed"),
            Err(error) => Err(error),
        }
    }
}

include!("private_file_temp_policy.rs");
