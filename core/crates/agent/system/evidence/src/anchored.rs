use crate::identity::{EvidenceIdentity, EvidenceKind};
use crate::layout::EvidenceLayout;
use crate::open::{directory_flags, entry_identity, file_flags, open_at, open_root};
use anyhow::Result;
use std::ffi::CString;
use std::fs::File;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};

pub(crate) struct AnchoredDirectory {
    pub(crate) file: File,
    pub(crate) identity: EvidenceIdentity,
    pub(crate) name: Option<CString>,
}

pub struct AnchoredEvidence {
    pub(crate) directories: Vec<AnchoredDirectory>,
    pub(crate) file: File,
    pub(crate) file_name: CString,
    pub(crate) identity: EvidenceIdentity,
    pub(crate) root_path: PathBuf,
    pub(crate) layout: EvidenceLayout,
}

impl AnchoredEvidence {
    pub fn open(root: &Path, path: &Path, layout: EvidenceLayout) -> Result<Option<Self>> {
        let Some(names) = layout.relative_names(root, path) else {
            return Ok(None);
        };
        let Some(root_file) = open_root(root)? else {
            return Ok(None);
        };
        let Some(root_identity) =
            EvidenceIdentity::from_metadata(&root_file.metadata()?, EvidenceKind::Directory)
        else {
            return Ok(None);
        };
        let mut directories = vec![AnchoredDirectory {
            file: root_file,
            identity: root_identity,
            name: None,
        }];
        let Some((file_name, directory_names)) = names.split_last() else {
            return Ok(None);
        };
        for name in directory_names {
            let Some(parent) = directories.last() else {
                return Ok(None);
            };
            let Some(file) = open_at(parent.file.as_raw_fd(), name, directory_flags())? else {
                return Ok(None);
            };
            let Some(identity) =
                EvidenceIdentity::from_metadata(&file.metadata()?, EvidenceKind::Directory)
            else {
                return Ok(None);
            };
            directories.push(AnchoredDirectory {
                file,
                identity,
                name: Some(name.clone()),
            });
        }
        let file_name = file_name.clone();
        let Some(parent) = directories.last() else {
            return Ok(None);
        };
        let Some(named) = entry_identity(
            parent.file.as_raw_fd(),
            &file_name,
            EvidenceKind::RegularFile,
        )?
        else {
            return Ok(None);
        };
        let Some(file) = open_at(parent.file.as_raw_fd(), &file_name, file_flags())? else {
            return Ok(None);
        };
        let Some(identity) =
            EvidenceIdentity::from_metadata(&file.metadata()?, EvidenceKind::RegularFile)
        else {
            return Ok(None);
        };
        if identity != named {
            return Ok(None);
        }
        Ok(Some(Self {
            directories,
            file,
            file_name,
            identity,
            root_path: root.to_path_buf(),
            layout,
        }))
    }

    pub fn file_mut(&mut self) -> &mut File {
        &mut self.file
    }

    pub fn identity(&self) -> EvidenceIdentity {
        self.identity
    }

    pub fn layout(&self) -> EvidenceLayout {
        self.layout
    }

    pub fn durable_object_identity(&self) -> Result<Option<yolo_paths::identity::ObjectIdentity>> {
        yolo_paths::identity::ObjectIdentity::from_file(&self.file)
    }
}

#[path = "anchored_verify.rs"]
mod verify;
