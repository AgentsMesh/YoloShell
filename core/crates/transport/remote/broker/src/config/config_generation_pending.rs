use super::constants::*;
use anyhow::{Result, ensure};
use std::ffi::CStr;
use std::fs::File;
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

#[path = "config_generation_entries.rs"]
mod entries;
#[path = "config_generation_io.rs"]
mod io;
#[path = "config_generation_names.rs"]
mod names;
#[path = "config_generation_root.rs"]
mod root;
#[path = "config_generation_slot.rs"]
mod slot;
use io::GenerationIdentity;
pub(super) use names::entry_name;
pub(super) use root::{GenerationRoot, OpenedGeneration};

pub(super) struct PendingDirectory {
    slot: slot::Slot,
    directory: File,
    identity: GenerationIdentity,
    path: PathBuf,
    armed: bool,
}

impl PendingDirectory {
    pub(super) fn create(root: &GenerationRoot) -> Result<Self> {
        Self::create_using(root, |_| Ok(()))
    }

    fn create_using<F>(root: &GenerationRoot, mut before_open: F) -> Result<Self>
    where
        F: FnMut(&CStr) -> Result<()>,
    {
        Self::create_with_hooks(root, &mut before_open, &mut |_| Ok(()))
    }

    fn create_with_hooks(
        root: &GenerationRoot,
        before_open: &mut impl FnMut(&CStr) -> Result<()>,
        after_open: &mut impl FnMut(&CStr) -> Result<()>,
    ) -> Result<Self> {
        for index in 0..MAX_CONCURRENT_GENERATION_STAGES {
            let name = entry_name(format!("{PENDING_GENERATION_PREFIX}{index}"))?;
            let Some(slot) = slot::Slot::claim(root, name, before_open, after_open)? else {
                continue;
            };
            let name = slot.payload_name();
            if io::create_directory_at(root.descriptor(), name)? {
                root.sync_and_verify()?;
            }
            let directory = io::open_directory_at(root.descriptor(), name)?;
            let path = root
                .path()
                .join(std::ffi::OsStr::from_bytes(name.to_bytes()));
            let initial_mode = directory.metadata()?.permissions().mode() & PERMISSION_BITS_MASK;
            ensure!(
                initial_mode == PRIVATE_DIRECTORY_MODE || initial_mode == IMMUTABLE_DIRECTORY_MODE,
                "broker generation staging payload has unsafe permissions"
            );
            let initial = GenerationIdentity::inspect(&directory.metadata()?, initial_mode)?;
            slot.verify_payload_identity(root, initial)?;
            io::set_directory_mode(&directory, &path, PRIVATE_DIRECTORY_MODE)?;
            let identity =
                GenerationIdentity::inspect(&directory.metadata()?, PRIVATE_DIRECTORY_MODE)?;
            let pending = Self {
                directory,
                slot,
                identity,
                path,
                armed: true,
            };
            pending.verify_source(root, PRIVATE_DIRECTORY_MODE)?;
            pending.clean_known_files();
            pending.directory.sync_all()?;
            entries::ensure_empty(&pending.directory)?;
            pending.verify_source(root, PRIVATE_DIRECTORY_MODE)?;
            return Ok(pending);
        }
        anyhow::bail!("broker generation staging capacity exhausted")
    }

    pub(super) fn write_file(&self, name: &CStr, bytes: &[u8]) -> Result<()> {
        ensure!(
            names::is_generation_file_name(name),
            "invalid broker generation file name"
        );
        let path = self.path.join(std::ffi::OsStr::from_bytes(name.to_bytes()));
        io::write_file_at(self.directory.as_raw_fd(), name, &path, bytes)
    }

    pub(super) fn seal(&self, root: &GenerationRoot) -> Result<()> {
        self.verify_source(root, PRIVATE_DIRECTORY_MODE)?;
        entries::ensure_known(&self.directory, &self.path)?;
        self.directory.sync_all()?;
        io::set_directory_mode(&self.directory, &self.path, IMMUTABLE_DIRECTORY_MODE)?;
        self.verify_source(root, IMMUTABLE_DIRECTORY_MODE)
    }

    pub(super) fn verify_source(&self, root: &GenerationRoot, mode: u32) -> Result<()> {
        crate::private_acl::require_empty(&self.directory, &self.path)?;
        let current = GenerationIdentity::inspect(&self.directory.metadata()?, mode)?;
        ensure!(
            current == self.identity,
            "broker generation staging descriptor changed"
        );
        self.slot.verify_payload_identity(root, self.identity)
    }

    pub(super) fn publish(&mut self, root: &GenerationRoot, target: &CStr) -> Result<bool> {
        self.verify_source(root, IMMUTABLE_DIRECTORY_MODE)?;
        let published =
            io::rename_exclusive_at(root.descriptor(), self.slot.payload_name(), target)?;
        if published {
            self.armed = false;
        }
        Ok(published)
    }

    pub(super) fn matches(&self, opened: &OpenedGeneration) -> bool {
        self.identity == opened.identity
    }

    fn clean_known_files(&self) {
        for name in names::generation_file_names() {
            io::unlink_file_at(self.directory.as_raw_fd(), &name);
        }
    }
}

impl Drop for PendingDirectory {
    fn drop(&mut self) {
        let descriptor_matches = self
            .directory
            .metadata()
            .ok()
            .is_some_and(|metadata| self.identity.matches(&metadata));
        if !self.armed || !descriptor_matches {
            return;
        }
        if io::set_directory_mode(&self.directory, &self.path, PRIVATE_DIRECTORY_MODE).is_ok() {
            self.clean_known_files();
            let _ = self.directory.sync_all();
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/config/config_generation_io_branch_tests.rs"]
mod io_branch_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/config/config_generation_root_tests.rs"]
mod root_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/config/config_generation_pending_tests.rs"]
mod tests;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/config/config_generation_slot_tests.rs"]
mod slot_tests;

#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/config/config_generation_acl_pending_tests.rs"]
mod acl_tests;
