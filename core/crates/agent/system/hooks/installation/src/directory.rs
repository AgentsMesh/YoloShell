use crate::{constants::*, unix};
use anyhow::{Result, ensure};
use serde::Serialize;
use std::fs::File;
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use yolo_paths::identity::ObjectIdentity;

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) struct DirectoryIdentity {
    object: ObjectIdentity,
    mode: u32,
    owner: u32,
}

pub(crate) struct Directory {
    file: File,
    path: PathBuf,
    identity: DirectoryIdentity,
}

impl Directory {
    pub fn open(path: &Path) -> Result<Self> {
        yolo_paths::validation::validate_absolute(path)?;
        let path = yolo_paths::platform::platform_path(path);
        ensure!(path.parent().is_some(), INVALID_HOME);
        let mut file =
            unix::open_directory(libc::AT_FDCWD, Path::new(std::path::MAIN_SEPARATOR_STR))?;
        let mut current = PathBuf::from(std::path::MAIN_SEPARATOR_STR);
        for component in path.components().skip(1) {
            let Component::Normal(name) = component else {
                anyhow::bail!(INVALID_HOME)
            };
            file = unix::open_directory(file.as_raw_fd(), Path::new(name))?;
            current.push(name);
            let metadata = file.metadata()?;
            let owner = metadata.uid();
            let mode = metadata.mode();
            let sticky_root = owner == ROOT_UID && mode & SPECIAL_MODE == STICKY_MODE;
            ensure!(
                owner == ROOT_UID || owner == unsafe { libc::geteuid() },
                UNSAFE_CONFIG
            );
            ensure!(
                sticky_root || mode & (UNSAFE_WRITE_MODE | SPECIAL_MODE) == 0,
                UNSAFE_CONFIG
            );
            yolo_paths::private_acl::require_no_grants(&file, &current)?;
        }
        let metadata = file.metadata()?;
        ensure!(metadata.uid() == unsafe { libc::geteuid() }, UNSAFE_CONFIG);
        ensure!(
            metadata.mode() & (UNSAFE_WRITE_MODE | SPECIAL_MODE) == 0,
            UNSAFE_CONFIG
        );
        let identity = DirectoryIdentity {
            object: ObjectIdentity::from_metadata(&metadata),
            mode: metadata.mode() & PERMISSION_MASK,
            owner: metadata.uid(),
        };
        Ok(Self {
            file,
            path,
            identity,
        })
    }

    pub fn file(&self) -> &File {
        &self.file
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn identity(&self) -> DirectoryIdentity {
        self.identity
    }

    pub fn verify(&self) -> Result<()> {
        let current = Self::open(&self.path)?;
        ensure!(current.identity == self.identity, CONFIG_CHANGED);
        let retained = self.file.metadata()?;
        ensure!(
            ObjectIdentity::from_metadata(&retained) == self.identity.object,
            CONFIG_CHANGED
        );
        ensure!(
            retained.mode() & PERMISSION_MASK == self.identity.mode,
            CONFIG_CHANGED
        );
        Ok(())
    }
}

/// The provider's configuration directory and every component below it that
/// does not exist yet. A provider that keeps its hook file in a subdirectory
/// (for example `~/.grok/hooks`) has not created its root before its CLI runs,
/// so the installer creates the missing chain one staging directory at a time
/// under the nearest existing ancestor.
fn missing_chain(home: &Path) -> Result<(Directory, Vec<String>)> {
    yolo_paths::validation::validate_absolute(home)?;
    let mut missing = Vec::new();
    let mut current = home.to_path_buf();
    while let Some(parent) = current.parent() {
        match std::fs::symlink_metadata(&current) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let name = current
                    .file_name()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| anyhow::anyhow!(INVALID_HOME))?;
                missing.push(name.to_owned());
                current = parent.to_path_buf();
            }
            Err(error) => return Err(error.into()),
            Ok(_) => break,
        }
    }
    ensure!(!missing.is_empty(), CONFIG_CHANGED);
    missing.reverse();
    Ok((Directory::open(&current)?, missing))
}

pub(crate) fn missing_revision(home: &Path, file_name: &str) -> Result<String> {
    let (base, missing) = missing_chain(home)?;
    crate::filesystem::revision(&(ABSENT_REVISION, home, file_name, base.identity(), &missing))
}

pub(crate) fn create_missing(home: &Path, file_name: &str, expected: &str) -> Result<Directory> {
    let (mut directory, missing) = missing_chain(home)?;
    ensure!(
        crate::filesystem::revision(&(
            ABSENT_REVISION,
            home,
            file_name,
            directory.identity(),
            &missing
        ))? == expected,
        CONFIG_CHANGED
    );
    for name in missing {
        directory.verify()?;
        let stage = format!("{STAGING_PREFIX}{}", uuid::Uuid::new_v4());
        unix::create_directory(directory.file(), Path::new(&stage))?;
        let retained = Directory::open(&directory.path().join(&stage))?;
        directory.verify()?;
        unix::rename(directory.file(), &stage, &name, false)?;
        directory.verify()?;
        let created = Directory::open(&directory.path().join(&name))?;
        ensure!(created.identity == retained.identity, CONFIG_CHANGED);
        directory = created;
    }
    Ok(directory)
}
