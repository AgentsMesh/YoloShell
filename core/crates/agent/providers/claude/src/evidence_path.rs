use super::constants;
use super::evidence_identity::{EvidenceIdentity, EvidenceKind};
use super::evidence_open::{directory_flags, entry_identity, file_flags, open_at, open_root};
use anyhow::{Context, Result};
use std::ffi::CString;
use std::fs::File;
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path, PathBuf};

const EVIDENCE_RELATIVE_COMPONENTS: usize = 3;

struct AnchoredDirectory {
    file: File,
    identity: EvidenceIdentity,
    name: Option<CString>,
}

pub(super) struct AnchoredEvidence {
    directories: Vec<AnchoredDirectory>,
    file: File,
    file_name: CString,
    identity: EvidenceIdentity,
    root_path: PathBuf,
}

impl AnchoredEvidence {
    pub(super) fn open(root: &Path, path: &Path) -> Result<Option<Self>> {
        let Some(names) = relative_names(root, path) else {
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
        }))
    }

    pub(super) fn file_mut(&mut self) -> &mut File {
        &mut self.file
    }

    pub(super) fn identity(&self) -> EvidenceIdentity {
        self.identity
    }

    pub(super) fn durable_object_identity(
        &self,
    ) -> Result<Option<crate::paths::identity::ObjectIdentity>> {
        crate::paths::identity::ObjectIdentity::from_file(&self.file)
    }

    pub(super) fn verify(&self) -> Result<bool> {
        self.verify_contents(true)
    }

    pub(super) fn verify_object(&self) -> Result<bool> {
        self.verify_contents(false)
    }

    fn verify_contents(&self, content: bool) -> Result<bool> {
        let Some(held) =
            EvidenceIdentity::from_metadata(&self.file.metadata()?, EvidenceKind::RegularFile)
        else {
            return Ok(false);
        };
        if !held.same_object(self.identity)
            || (content && held != self.identity)
            || !self.verify_root()?
        {
            return Ok(false);
        }
        for pair in self.directories.windows(2) {
            let child = &pair[1];
            let Some(name) = child.name.as_ref() else {
                return Ok(false);
            };
            let Some(named) =
                entry_identity(pair[0].file.as_raw_fd(), name, EvidenceKind::Directory)?
            else {
                return Ok(false);
            };
            if !named.same_object(child.identity) {
                return Ok(false);
            }
        }
        let Some(parent) = self.directories.last() else {
            return Ok(false);
        };
        Ok(entry_identity(
            parent.file.as_raw_fd(),
            &self.file_name,
            EvidenceKind::RegularFile,
        )?
        .is_some_and(|named| {
            named.same_object(self.identity) && (!content || named == self.identity)
        }))
    }

    fn verify_root(&self) -> Result<bool> {
        let metadata = match std::fs::symlink_metadata(&self.root_path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error).context(constants::DATA_HOME_INSPECT_ERROR),
        };
        Ok(
            EvidenceIdentity::from_metadata(&metadata, EvidenceKind::Directory)
                .is_some_and(|named| named.same_object(self.directories[0].identity)),
        )
    }
}

fn relative_names(root: &Path, path: &Path) -> Option<Vec<CString>> {
    if !root.is_absolute() || root.components().any(|part| part == Component::ParentDir) {
        return None;
    }
    let relative = path.strip_prefix(root).ok()?;
    let names = relative
        .components()
        .map(|part| match part {
            Component::Normal(name) => CString::new(name.as_bytes()).ok(),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    (names.len() == EVIDENCE_RELATIVE_COMPONENTS
        && names
            .first()
            .is_some_and(|name| name.as_bytes() == constants::EVIDENCE_DIR.as_bytes()))
    .then_some(names)
}
