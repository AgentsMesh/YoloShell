use super::private_file_io::{open_named, read_flags, rename_exclusive_at};
use super::private_file_missing::retained_entry_is_missing;
use super::private_file_names::retained_isolation_name;
use super::private_file_retained::{RetainedCleanup, RetainedPrivateFile};
use super::private_file_retained_hooks::*;
use super::*;

impl PrivateFileEntry {
    pub fn install_retained<F>(
        &self,
        temporary: &mut RetainedPrivateFile,
        expected: FileIdentity,
        max_bytes: u64,
        mut fence: F,
    ) -> Result<FileIdentity>
    where
        F: FnMut() -> Result<()>,
    {
        self.verify_retained_content(expected, max_bytes)?;
        temporary.verify_named(max_bytes)?;
        run_isolation_hook()?;
        self.verify_retained_content(expected, max_bytes)?;
        temporary.verify_named(max_bytes)?;
        let retired = retained_isolation_name(&self.name)?;
        rename_exclusive_at(self.directory.descriptor(), &self.name, &retired)
            .context("isolate retained target")?;
        let displaced = self.open_sibling(&retired, max_bytes)?;
        if !displaced.identity.same_content_version(expected) {
            temporary.preserve();
            anyhow::bail!("isolated retained target changed");
        }
        temporary.replace_cleanup(retired.clone(), displaced.identity);
        let prepared = self
            .prepare_retained_install(temporary, &mut fence, max_bytes)
            .and_then(|()| run_rename_hook());
        if let Err(error) = prepared {
            return self.abort_retained_install(temporary, &retired, displaced.identity, error);
        }
        if let Err(error) =
            rename_exclusive_at(self.directory.descriptor(), &temporary.name, &self.name)
        {
            return self.abort_retained_install(
                temporary,
                &retired,
                displaced.identity,
                error.into(),
            );
        }
        match self.verify_installed_retained(temporary, max_bytes) {
            Ok(identity) => Ok(identity),
            Err(error) => {
                temporary.preserve();
                Err(error.context("verify installed retained target"))
            }
        }
    }

    pub fn rollback_retained_install(
        &self,
        temporary: &mut RetainedPrivateFile,
        installed: FileIdentity,
        previous: FileIdentity,
        max_bytes: u64,
    ) -> Result<()> {
        let result =
            self.rollback_retained_install_inner(temporary, installed, previous, max_bytes);
        if result.is_err() {
            temporary.preserve();
        }
        result
    }

    fn rollback_retained_install_inner(
        &self,
        temporary: &mut RetainedPrivateFile,
        installed: FileIdentity,
        previous: FileIdentity,
        max_bytes: u64,
    ) -> Result<()> {
        let RetainedCleanup::Named(names) = &temporary.cleanup else {
            anyhow::bail!("retained rollback has no isolated target");
        };
        let retired = names
            .first()
            .context("retained rollback target is missing")?;
        let retired_name = retired.name.clone();
        self.verify_retained_content(installed, max_bytes)?;
        let old = self.open_sibling(&retired_name, max_bytes)?;
        anyhow::ensure!(
            old.identity.same_content_version(previous),
            "old cursor changed"
        );
        rename_exclusive_at(self.directory.descriptor(), &self.name, &temporary.name)
            .context("isolate installed retained target")?;
        run_restore_hook()?;
        rename_exclusive_at(self.directory.descriptor(), &retired_name, &self.name)
            .context("restore retained target")?;
        temporary.preserve();
        self.verify_retained_content(previous, max_bytes)?;
        temporary.verify_named_content(installed, max_bytes)?;
        Ok(())
    }

    fn prepare_retained_install<F>(
        &self,
        temporary: &RetainedPrivateFile,
        fence: &mut F,
        max_bytes: u64,
    ) -> Result<()>
    where
        F: FnMut() -> Result<()>,
    {
        fence()?;
        run_install_hook()?;
        fence()?;
        temporary.verify_named(max_bytes)?;
        anyhow::ensure!(
            retained_entry_is_missing(&self.directory, &self.name)?,
            "retained target was replaced before install"
        );
        Ok(())
    }

    fn verify_installed_retained(
        &self,
        temporary: &RetainedPrivateFile,
        max_bytes: u64,
    ) -> Result<FileIdentity> {
        let descriptor =
            FileIdentity::inspect_file_bounded(&temporary.file, &temporary.path, max_bytes)?;
        anyhow::ensure!(
            descriptor.same_content_version(temporary.identity),
            "installed retained descriptor changed"
        );
        let installed = self.verify_retained_content(temporary.identity, max_bytes)?;
        Ok(installed.identity)
    }

    fn abort_retained_install(
        &self,
        temporary: &mut RetainedPrivateFile,
        retired: &std::ffi::CStr,
        previous: FileIdentity,
        cause: anyhow::Error,
    ) -> Result<FileIdentity> {
        if let Err(error) = self.rollback_isolation(temporary, retired, previous) {
            temporary.preserve();
            return Err(cause.context(format!("retained target could not be restored: {error:#}")));
        }
        Err(cause)
    }

    fn rollback_isolation(
        &self,
        temporary: &mut RetainedPrivateFile,
        retired: &std::ffi::CStr,
        expected: FileIdentity,
    ) -> Result<()> {
        anyhow::ensure!(
            retained_entry_is_missing(&self.directory, &self.name)?,
            "retained target is occupied"
        );
        rename_exclusive_at(self.directory.descriptor(), retired, &self.name)
            .context("restore isolated retained target")?;
        temporary.preserve();
        self.verify_retained_content(expected, expected.length())?;
        self.sync_retained_parent()
    }

    fn open_sibling(&self, name: &std::ffi::CStr, max_bytes: u64) -> Result<OpenedPrivateFile> {
        open_named(
            self.directory.descriptor(),
            name,
            self.path(),
            read_flags(),
            max_bytes,
        )
    }
}
