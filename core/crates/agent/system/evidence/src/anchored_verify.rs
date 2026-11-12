use super::AnchoredEvidence;
use crate::constants::message;
use crate::identity::{EvidenceIdentity, EvidenceKind};
use crate::open::entry_identity;
use anyhow::{Context, Result};
use std::os::fd::AsRawFd;

impl AnchoredEvidence {
    pub fn verify(&self) -> Result<bool> {
        self.verify_contents(true)
    }

    pub fn verify_object(&self) -> Result<bool> {
        self.verify_contents(false)
    }

    fn verify_contents(&self, content: bool) -> Result<bool> {
        let Some(held) =
            EvidenceIdentity::from_metadata(&self.file.metadata()?, EvidenceKind::RegularFile)
        else {
            return Ok(false);
        };
        if !held.same_object(self.identity) || (content && held != self.identity) {
            return Ok(false);
        }
        if !self.verify_root()? {
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
            Err(error) => return Err(error).context(message::DATA_HOME_INSPECT),
        };
        Ok(
            EvidenceIdentity::from_metadata(&metadata, EvidenceKind::Directory)
                .is_some_and(|named| named.same_object(self.directories[0].identity)),
        )
    }
}
