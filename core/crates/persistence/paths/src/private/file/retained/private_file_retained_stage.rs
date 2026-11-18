use super::private_file_clone::clone_private_file_at;
use super::private_file_io::{lock_private_file, open_named, read_flags, write_flags};
use super::private_file_names::retired_name;
use super::private_file_retained::{RetainedCleanup, RetainedPrivateFile};
use super::*;
use std::io::{Seek, SeekFrom, Write};
use std::os::unix::fs::FileExt;

#[derive(Debug)]
pub struct RetainedWrite {
    pub bytes: Vec<u8>,
    pub offset: u64,
}

impl PrivateFileEntry {
    pub fn stage_retained<F, T>(
        &self,
        max_bytes: u64,
        prepare: F,
    ) -> Result<(RetainedPrivateFile, T)>
    where
        F: FnOnce(&File, FileIdentity) -> Result<(Vec<RetainedWrite>, T)>,
    {
        let name = retired_name(&self.name)?;
        let file = self.open_retained_stage(&name, max_bytes)?;
        let process_lock = lock_private_file(&file)?;
        let initial = FileIdentity::inspect_file_bounded(&file, &self.path, max_bytes)?;
        let (writes, value) = prepare(&file, initial)?;
        anyhow::ensure!(!writes.is_empty(), "retained staging has no writes");
        validate_retained_writes(&writes, max_bytes)?;
        self.verify_retained_stage(&name, initial, max_bytes)?;
        let mut temporary = RetainedPrivateFile {
            cleanup: RetainedCleanup::PreserveSyncPending,
            directory: self.directory.try_clone()?,
            file,
            _process_lock: process_lock,
            identity: initial,
            name,
            path: self.path.clone(),
        };
        for write in &writes {
            temporary.file.seek(SeekFrom::Start(write.offset))?;
            temporary.file.write_all(&write.bytes)?;
            temporary.file.sync_all().context("sync retained staging")?;
            temporary.identity =
                FileIdentity::inspect_file_bounded(&temporary.file, &self.path, max_bytes)?;
            temporary.verify_named(max_bytes)?;
            verify_retained_write(&temporary, write, max_bytes)?;
        }
        self.directory.sync()?;
        Ok((temporary, value))
    }

    fn open_retained_stage(&self, name: &CString, max: u64) -> Result<File> {
        match open_named(
            self.directory.descriptor(),
            name,
            &self.path,
            write_flags(),
            max,
        ) {
            Ok(opened) => Ok(opened.file),
            Err(error) if io_kind(&error) == Some(std::io::ErrorKind::NotFound) => {
                let source = self.open_retained(max)?;
                clone_private_file_at(&source.file, &self.directory, name, &self.path, max)?;
                self.verify_retained_content(source.identity, max)?;
                let opened = open_named(
                    self.directory.descriptor(),
                    name,
                    &self.path,
                    write_flags(),
                    max,
                )?;
                Ok(opened.file)
            }
            Err(error) => Err(error),
        }
    }

    fn verify_retained_stage(
        &self,
        name: &CString,
        expected: FileIdentity,
        max_bytes: u64,
    ) -> Result<()> {
        let opened = open_named(
            self.directory.descriptor(),
            name,
            &self.path,
            read_flags(),
            max_bytes,
        )?;
        anyhow::ensure!(
            opened.identity.unchanged(expected),
            "reusable retained staging changed"
        );
        Ok(())
    }
}

fn validate_retained_writes(writes: &[RetainedWrite], max_bytes: u64) -> Result<()> {
    for write in writes {
        let write_end = write
            .offset
            .checked_add(u64::try_from(write.bytes.len())?)
            .context("retained staging length overflow")?;
        anyhow::ensure!(
            write_end <= max_bytes,
            "retained staging exceeds byte limit"
        );
    }
    Ok(())
}

fn verify_retained_write(
    temporary: &RetainedPrivateFile,
    write: &RetainedWrite,
    max_bytes: u64,
) -> Result<()> {
    let mut actual = vec![0_u8; write.bytes.len()];
    temporary.file.read_exact_at(&mut actual, write.offset)?;
    anyhow::ensure!(actual == write.bytes, "retained staging write changed");
    temporary.identity.verify_file_bounded(
        &temporary.file,
        temporary.identity.length(),
        &temporary.path,
        max_bytes,
    )
}
