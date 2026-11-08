use crate::{constants::*, descriptor::*};
use std::ffi::CString;
use std::fs::File;
use std::os::fd::AsRawFd;
use yolo_hub_terminal_protocol::PublicError;
use yolo_paths::identity::ObjectIdentity;

struct Directory {
    file: File,
    identity: ObjectIdentity,
    name: CString,
}

pub(crate) struct Anchor {
    directories: Vec<Directory>,
}

impl Anchor {
    pub fn object_identity(&self) -> Result<ObjectIdentity, PublicError> {
        self.directories
            .last()
            .map(|directory| directory.identity)
            .ok_or(PublicError::Internal)
    }

    pub fn absolute(path: &str) -> Result<Self, PublicError> {
        let relative = path
            .strip_prefix(ROOT_PATH)
            .ok_or(PublicError::InvalidMetadata)?;
        let name = CString::new(ROOT_PATH).map_err(|_| PublicError::Internal)?;
        let file = open_at(libc::AT_FDCWD, &name, DIRECTORY_FLAGS)?;
        let root = Directory {
            identity: identity(&file)?,
            file,
            name,
        };
        let mut anchor = Self {
            directories: vec![root],
        };
        for name in relative.split(PATH_SEPARATOR) {
            anchor.push(name)?;
        }
        anchor.verify()?;
        Ok(anchor)
    }

    pub fn clone_anchor(&self) -> Result<Self, PublicError> {
        let directories = self
            .directories
            .iter()
            .map(|directory| {
                Ok(Directory {
                    file: directory.file.try_clone().map_err(io_error)?,
                    identity: directory.identity,
                    name: directory.name.clone(),
                })
            })
            .collect::<Result<_, PublicError>>()?;
        Ok(Self { directories })
    }

    pub fn push(&mut self, name: &str) -> Result<(), PublicError> {
        let name = CString::new(name).map_err(|_| PublicError::InvalidMetadata)?;
        let file = self.open(&name, DIRECTORY_FLAGS)?;
        self.directories.push(Directory {
            identity: identity(&file)?,
            file,
            name,
        });
        Ok(())
    }

    pub fn open(&self, name: &CString, flags: libc::c_int) -> Result<File, PublicError> {
        let parent = self.directories.last().ok_or(PublicError::Internal)?;
        open_at(parent.file.as_raw_fd(), name, flags)
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        let last = self.directories.last().map(|value| value.identity);
        let other_last = other.directories.last().map(|value| value.identity);
        self.directories
            .iter()
            .any(|value| Some(value.identity) == other_last)
            || other
                .directories
                .iter()
                .any(|value| Some(value.identity) == last)
    }

    pub fn verify(&self) -> Result<(), PublicError> {
        for (index, directory) in self.directories.iter().enumerate() {
            let parent = index
                .checked_sub(1)
                .map(|previous| self.directories[previous].file.as_raw_fd())
                .unwrap_or(libc::AT_FDCWD);
            let named = open_at(parent, &directory.name, DIRECTORY_FLAGS)
                .map_err(|_| PublicError::Conflict)?;
            if identity(&named)? != directory.identity
                || identity(&directory.file)? != directory.identity
            {
                return Err(PublicError::Conflict);
            }
        }
        Ok(())
    }
}
