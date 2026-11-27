use super::*;
use anyhow::{Context, Error, Result};
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;

const ROLLBACK_CONTEXT: &str = "rollback isolated test socket root creation";

#[derive(Clone, Copy, Eq, PartialEq)]
struct CreatedIdentity {
    device: libc::dev_t,
    inode: libc::ino_t,
}

pub(super) struct CreationRollback<'a> {
    parent: &'a File,
    name: &'a CString,
    identity: CreatedIdentity,
    armed: bool,
}

impl<'a> CreationRollback<'a> {
    pub(super) fn capture(parent: &'a File, name: &'a CString) -> Result<Self> {
        let metadata = metadata_at(parent, name)?;
        anyhow::ensure!(
            file_type(metadata.st_mode) == libc::S_IFDIR,
            "created socket root is not a directory"
        );
        anyhow::ensure!(
            metadata.st_uid == unsafe { libc::geteuid() },
            "created socket root owner mismatch"
        );
        let identity = CreatedIdentity {
            device: metadata.st_dev,
            inode: metadata.st_ino,
        };
        Ok(Self {
            parent,
            name,
            identity,
            armed: true,
        })
    }

    pub(super) fn disarm(&mut self) {
        self.armed = false;
    }

    pub(super) fn verify_opened(&self, opened: &File) -> Result<()> {
        let metadata = opened.metadata()?;
        let current = CreatedIdentity {
            device: libc::dev_t::try_from(metadata.dev())?,
            inode: libc::ino_t::try_from(metadata.ino())?,
        };
        anyhow::ensure!(
            current == self.identity,
            "created socket root identity changed"
        );
        Ok(())
    }

    pub(super) fn verify_named(&self) -> Result<()> {
        let metadata = metadata_at(self.parent, self.name)?;
        let current = CreatedIdentity {
            device: metadata.st_dev,
            inode: metadata.st_ino,
        };
        anyhow::ensure!(
            current == self.identity,
            "created socket root identity changed"
        );
        anyhow::ensure!(
            file_type(metadata.st_mode) == libc::S_IFDIR,
            "created socket root type changed"
        );
        Ok(())
    }

    pub(super) fn fail<T>(mut self, original: Error) -> Result<T> {
        match self.rollback() {
            Ok(()) => Err(original),
            Err(rollback) => Err(original.context(format!("{ROLLBACK_CONTEXT}: {rollback:#}"))),
        }
    }

    fn rollback(&mut self) -> Result<()> {
        if !self.armed {
            return Ok(());
        }
        let metadata = metadata_at(self.parent, self.name)?;
        let current = CreatedIdentity {
            device: metadata.st_dev,
            inode: metadata.st_ino,
        };
        anyhow::ensure!(
            current == self.identity,
            "created socket root identity changed"
        );
        anyhow::ensure!(
            file_type(metadata.st_mode) == libc::S_IFDIR,
            "created socket root type changed"
        );
        let result = unsafe {
            libc::unlinkat(
                self.parent.as_raw_fd(),
                self.name.as_ptr(),
                libc::AT_REMOVEDIR,
            )
        };
        anyhow::ensure!(result == 0, std::io::Error::last_os_error());
        self.parent.sync_all().context(ROLLBACK_CONTEXT)?;
        self.armed = false;
        Ok(())
    }
}

impl Drop for CreationRollback<'_> {
    fn drop(&mut self) {
        let _ = self.rollback();
    }
}

fn metadata_at(parent: &File, name: &CString) -> Result<libc::stat> {
    let mut metadata = unsafe { std::mem::zeroed::<libc::stat>() };
    let result = unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            &mut metadata,
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    anyhow::ensure!(result == 0, std::io::Error::last_os_error());
    Ok(metadata)
}

fn file_type(mode: libc::mode_t) -> libc::mode_t {
    mode & libc::S_IFMT
}
