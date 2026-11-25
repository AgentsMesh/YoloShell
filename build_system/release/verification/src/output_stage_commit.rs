use crate::output_stage::StagedCandidate;
use anyhow::{Context, Result, anyhow, ensure};
use std::ffi::CString;
use yolo_release_contracts::{NO_FILE_FLAGS, PRIVATE_DIRECTORY_MODE, SEALED_DIRECTORY_MODE};
use yolo_release_filesystem::DirectoryHandle;

pub(crate) const ROLLBACK_AUTHORITY_ERROR: &str = "candidate rollback lost destination authority";

impl StagedCandidate {
    pub(crate) fn install_name(&mut self, destination: &str) -> Result<()> {
        let destination = CString::new(destination)?;
        rename(&self.parent, &self.name, &destination, "install candidate")?;
        self.installed = true;
        Ok(())
    }

    pub(crate) fn protect_commit_mode(&self) -> Result<()> {
        self.handle.set_mode(SEALED_DIRECTORY_MODE)
    }

    pub(crate) fn apply_commit_marker(&self) -> Result<()> {
        self.handle.set_flags(libc::UF_IMMUTABLE)
    }

    pub(crate) fn confirm_durable_commit(&mut self) {
        debug_assert!(self.installed);
        self.committed = true;
    }

    pub(crate) fn rollback(&mut self, destination: &str) -> Result<()> {
        self.rollback_cstring(destination, &CString::new(destination)?)
    }

    fn rollback_cstring(&mut self, destination_text: &str, destination: &CString) -> Result<()> {
        ensure!(self.installed, "candidate directory is not installed");
        ensure!(!self.committed, "committed candidate cannot be rolled back");
        let flags = self.handle.set_flags(NO_FILE_FLAGS);
        let mode = self.handle.set_mode(PRIVATE_DIRECTORY_MODE);
        let authority = self
            .parent
            .verify_child(destination_text, &self.handle)
            .context(ROLLBACK_AUTHORITY_ERROR);
        if let Err(error) = authority {
            return Err(rollback_error(error, flags, mode));
        }
        if let Err(error) = rename(&self.parent, destination, &self.name, "rollback candidate") {
            let _ = self.handle.set_mode(SEALED_DIRECTORY_MODE);
            return Err(rollback_error(error, flags, mode));
        }
        self.installed = false;
        let staged_name = self.name.to_str().context(ROLLBACK_AUTHORITY_ERROR)?;
        self.parent
            .verify_child(staged_name, &self.handle)
            .context(ROLLBACK_AUTHORITY_ERROR)?;
        let sync = self
            .parent
            .durability_barrier()
            .context("sync candidate rollback");
        match (flags, mode, sync) {
            (Ok(()), Ok(()), Ok(())) => Ok(()),
            (flags, mode, sync) => Err(anyhow!(
                "candidate rollback protection: flags={flags:?}, mode={mode:?}, sync={sync:?}"
            )),
        }
    }
}

fn rollback_error(rename: anyhow::Error, flags: Result<()>, mode: Result<()>) -> anyhow::Error {
    anyhow!("rollback candidate: {rename:#}; flags={flags:?}; mode={mode:?}")
}

fn rename(
    parent: &DirectoryHandle,
    source: &CString,
    destination: &CString,
    description: &str,
) -> Result<()> {
    let status = unsafe {
        libc::renameatx_np(
            parent.raw_fd(),
            source.as_ptr(),
            parent.raw_fd(),
            destination.as_ptr(),
            libc::RENAME_EXCL,
        )
    };
    if status != 0 {
        return Err(std::io::Error::last_os_error()).context(description.to_owned());
    }
    Ok(())
}
