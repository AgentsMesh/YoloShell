use super::super::constants::*;
use super::{GenerationRoot, io, io::GenerationIdentity};
use anyhow::{Context, Result, ensure};
use std::ffi::{CStr, CString};
use std::fs::File;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;

const SLOT_DESCRIPTOR_CHANGED: &str = "broker staging slot descriptor changed";

/// A staging slot owns the lock but never becomes a published generation.
/// Its named sibling payload moves within the root: a sealed directory cannot
/// change parents on macOS. Late openers never acquire a published payload.
pub(super) struct Slot {
    _lock: crate::paths::process_file_lock::ProcessFileLock,
    directory: File,
    identity: GenerationIdentity,
    name: CString,
    payload_name: CString,
    path: PathBuf,
}

impl Slot {
    pub(super) fn claim(
        root: &GenerationRoot,
        name: CString,
        before_open: &mut impl FnMut(&CStr) -> Result<()>,
        after_open: &mut impl FnMut(&CStr) -> Result<()>,
    ) -> Result<Option<Self>> {
        root.verify_path()?;
        if io::create_directory_at(root.descriptor(), &name)? {
            root.sync_and_verify()?;
        }
        before_open(&name)?;
        let directory = match io::open_directory_at(root.descriptor(), &name) {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error).context("open broker generation staging slot"),
        };
        let identity = GenerationIdentity::inspect(&directory.metadata()?, PRIVATE_DIRECTORY_MODE)?;
        after_open(&name)?;
        let Some(lock) = io::try_lock(&directory)? else {
            return Ok(None);
        };
        ensure!(
            GenerationIdentity::inspect(&directory.metadata()?, PRIVATE_DIRECTORY_MODE)?
                == identity,
            SLOT_DESCRIPTOR_CHANGED
        );
        // An opened descriptor grants no cleanup/mode authority until the exact
        // fixed slot name has been checked under its exclusive lock.
        root.verify_identity(&name, identity)?;
        let path = root
            .path()
            .join(std::ffi::OsStr::from_bytes(name.to_bytes()));
        io::set_directory_mode(&directory, &path, PRIVATE_DIRECTORY_MODE)?;
        let slot = Self {
            _lock: lock,
            directory,
            identity,
            payload_name: super::names::pending_payload_name(&name)?,
            name,
            path,
        };
        slot.verify(root)?;
        super::entries::ensure_empty(&slot.directory)?;
        Ok(Some(slot))
    }

    pub(super) fn payload_name(&self) -> &CStr {
        &self.payload_name
    }

    pub(super) fn verify(&self, root: &GenerationRoot) -> Result<()> {
        crate::private_acl::require_empty(&self.directory, &self.path)?;
        let current =
            GenerationIdentity::inspect(&self.directory.metadata()?, PRIVATE_DIRECTORY_MODE)?;
        ensure!(current == self.identity, SLOT_DESCRIPTOR_CHANGED);
        root.verify_named(&self.name, self.identity, PRIVATE_DIRECTORY_MODE)
    }

    pub(super) fn verify_payload_identity(
        &self,
        root: &GenerationRoot,
        expected: GenerationIdentity,
    ) -> Result<()> {
        root.verify_identity(&self.payload_name, expected)?;
        self.verify(root)
    }
}
