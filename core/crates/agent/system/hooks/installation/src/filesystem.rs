use crate::{constants::*, directory::Directory, unix};
use anyhow::{Result, ensure};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::os::unix::fs::MetadataExt;
use yolo_paths::identity::ObjectIdentity;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct Identity {
    object: ObjectIdentity,
    length: u64,
    modified: (i64, i64),
    changed: (i64, i64),
    pub mode: u32,
}

pub(crate) struct Snapshot {
    pub bytes: Vec<u8>,
    pub document: Value,
    pub identity: Option<Identity>,
    pub revision: String,
}

impl Identity {
    fn inspect(file: &File) -> Result<Self> {
        let metadata = file.metadata()?;
        ensure!(
            metadata.is_file() && metadata.nlink() == SINGLE_LINK,
            UNSAFE_CONFIG
        );
        ensure!(metadata.uid() == unsafe { libc::geteuid() }, UNSAFE_CONFIG);
        let mode = metadata.mode() & PERMISSION_MASK;
        ensure!(
            mode & (UNSAFE_WRITE_MODE | SPECIAL_MODE) == 0,
            UNSAFE_CONFIG
        );
        ensure!(metadata.len() <= MAX_CONFIG_BYTES, INVALID_CONFIG);
        Ok(Self {
            object: ObjectIdentity::from_metadata(&metadata),
            length: metadata.len(),
            modified: (metadata.mtime(), metadata.mtime_nsec()),
            changed: (metadata.ctime(), metadata.ctime_nsec()),
            mode,
        })
    }

    pub fn after_rename(self, current: Self) -> bool {
        self.object == current.object
            && self.length == current.length
            && self.modified == current.modified
            && self.mode == current.mode
    }
}

pub(crate) fn read(directory: &Directory, name: &str) -> Result<Snapshot> {
    directory.verify()?;
    let (bytes, identity) = match unix::open_file(directory.file(), name) {
        Ok(file) => {
            yolo_paths::private_acl::require_empty(&file, &directory.path().join(name))?;
            let identity = Identity::inspect(&file)?;
            let mut bytes = Vec::new();
            (&file).take(MAX_CONFIG_BYTES + 1).read_to_end(&mut bytes)?;
            ensure!(Identity::inspect(&file)? == identity, CONFIG_CHANGED);
            let named = unix::open_file(directory.file(), name)?;
            ensure!(Identity::inspect(&named)? == identity, CONFIG_CHANGED);
            (bytes, Some(identity))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (Vec::new(), None),
        Err(error) => return Err(error.into()),
    };
    directory.verify()?;
    let document = if identity.is_some() {
        serde_json::from_slice(&bytes)?
    } else {
        serde_json::json!({})
    };
    crate::configuration::validate(&document)?;
    let revision = revision(&(
        directory.path(),
        directory.identity(),
        name,
        identity,
        &bytes,
    ))?;
    Ok(Snapshot {
        bytes,
        document,
        identity,
        revision,
    })
}

pub(crate) fn revision(value: &impl Serialize) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(value)?)))
}

pub(crate) fn verify(
    directory: &Directory,
    name: &str,
    snapshot: &Snapshot,
    moved: bool,
) -> Result<()> {
    let current = read(directory, name)?;
    let identities_match = match (snapshot.identity, current.identity) {
        (Some(previous), Some(actual)) if moved => previous.after_rename(actual),
        (previous, actual) => previous == actual,
    };
    ensure!(
        identities_match && current.bytes == snapshot.bytes,
        CONFIG_CHANGED
    );
    Ok(())
}
