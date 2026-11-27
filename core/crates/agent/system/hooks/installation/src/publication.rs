use crate::{
    constants::*,
    directory::Directory,
    filesystem::{self, Snapshot},
    unix,
};
use anyhow::{Result, ensure};
use std::io::Write;
use std::path::PathBuf;

pub(crate) fn backup(
    directory: &Directory,
    target: &str,
    snapshot: &Snapshot,
) -> Result<Option<PathBuf>> {
    if snapshot.identity.is_none() {
        return Ok(None);
    }
    let name = format!("{BACKUP_PREFIX}{}{BACKUP_SUFFIX}", snapshot.revision);
    filesystem::verify(directory, target, snapshot, false)?;
    match unix::create_file(directory.file(), &name, OWNER_FILE_MODE) {
        Ok(mut file) => {
            file.write_all(&snapshot.bytes)?;
            file.sync_all()?;
            directory.file().sync_all()?;
        }
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|error| error.kind() == std::io::ErrorKind::AlreadyExists) => {}
        Err(error) => return Err(error),
    }
    let saved = filesystem::read(directory, &name)?;
    ensure!(saved.bytes == snapshot.bytes, CONFIG_CHANGED);
    filesystem::verify(directory, target, snapshot, false)?;
    Ok(Some(directory.path().join(name)))
}

pub(crate) fn write(
    directory: &Directory,
    target: &str,
    snapshot: &Snapshot,
    bytes: &[u8],
) -> Result<()> {
    let stage = format!("{STAGING_PREFIX}{}", uuid::Uuid::new_v4());
    let mode = snapshot
        .identity
        .map_or(OWNER_FILE_MODE, |identity| identity.mode);
    let mut file = unix::create_file(directory.file(), &stage, mode)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    let staged = filesystem::read(directory, &stage)?;
    ensure!(staged.bytes == bytes, CONFIG_CHANGED);
    filesystem::verify(directory, target, snapshot, false)?;
    filesystem::verify(directory, &stage, &staged, false)?;
    let exchange = snapshot.identity.is_some();
    #[cfg(test)]
    tests::before_exchange();
    unix::rename(directory.file(), &stage, target, exchange)?;
    let validation = filesystem::verify(directory, target, &staged, true).and_then(|()| {
        if exchange {
            filesystem::verify(directory, &stage, snapshot, true)
        } else {
            Ok(())
        }
    });
    if let Err(error) = validation {
        // Never overwrite a new replacement while trying to roll back a contested exchange.
        filesystem::verify(directory, target, &staged, true)?;
        if exchange {
            let displaced = filesystem::read(directory, &stage)?;
            ensure!(displaced.identity.is_some(), CONFIG_CHANGED);
            filesystem::verify(directory, &stage, &displaced, false)?;
            unix::rename(directory.file(), &stage, target, true)?;
        } else {
            unix::rename(directory.file(), target, &stage, false)?;
        }
        return Err(error);
    }
    if exchange {
        filesystem::verify(directory, &stage, snapshot, true)?;
        unix::remove(directory.file(), &stage)?;
    }
    filesystem::verify(directory, target, &staged, true)
}

#[cfg(test)]
#[path = "../unit_tests/publication.rs"]
mod tests;
