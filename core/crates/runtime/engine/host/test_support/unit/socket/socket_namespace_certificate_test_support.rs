use crate::host::tests::*;
use crate::paths::directory::PrivateDirectory;
use anyhow::{Context, Result, ensure};
use std::{fs, path::PathBuf};

const ATTRIBUTE: &std::ffi::CStr = c"user.yolo.namespace";
// Match the current bounded directory/registration durable-proof certificate.
const MAX_CERTIFICATE_BYTES: usize = 2_048;
const CORRUPT_CERTIFICATE: &[u8] = b"{corrupt";
const SAVED_EXTENSION: &str = "retained";
const ATTRIBUTE_OPTIONS: libc::c_int = 0;
#[cfg(target_os = "macos")]
const ATTRIBUTE_POSITION: u32 = 0;
const ATTRIBUTE_ERROR: &str = "mutate retained test socket certificate";
const REPLACEMENT_CHANGED: &str = "test socket namespace replacement changed";

pub(super) struct CorruptCertificate {
    directory: PrivateDirectory,
    original: Vec<u8>,
}

impl CorruptCertificate {
    pub(super) fn new(directory: PrivateDirectory) -> Self {
        let original = read(&directory).unwrap();
        write(&directory, CORRUPT_CERTIFICATE).unwrap();
        Self {
            directory,
            original,
        }
    }
}

impl Drop for CorruptCertificate {
    fn drop(&mut self) {
        let _ = write(&self.directory, &self.original);
    }
}

pub(super) struct ReplacedDirectory {
    original: PrivateDirectory,
    replacement: PrivateDirectory,
    saved: PathBuf,
}

impl ReplacedDirectory {
    pub(super) fn new(original: PrivateDirectory) -> Self {
        let certificate = read(&original).unwrap();
        let saved = original.path().with_extension(SAVED_EXTENSION);
        fs::rename(original.path(), &saved).unwrap();
        let replacement = PrivateDirectory::ensure(original.path()).unwrap();
        write(&replacement, &certificate).unwrap();
        Self {
            original,
            replacement,
            saved,
        }
    }

    fn restore(&self) -> Result<()> {
        self.replacement.verify_path()?;
        let saved = PrivateDirectory::open(&self.saved)?;
        ensure!(saved.same_identity(&self.original), REPLACEMENT_CHANGED);
        fs::remove_dir(self.replacement.path())?;
        fs::rename(&self.saved, self.original.path())?;
        self.original.verify_path()
    }
}

impl Drop for ReplacedDirectory {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

pub(super) fn read(directory: &PrivateDirectory) -> Result<Vec<u8>> {
    directory.verify_path()?;
    let mut bytes = vec![u8::default(); MAX_CERTIFICATE_BYTES];
    #[cfg(target_os = "macos")]
    let count = unsafe {
        libc::fgetxattr(
            directory.descriptor(),
            ATTRIBUTE.as_ptr(),
            bytes.as_mut_ptr().cast(),
            bytes.len(),
            ATTRIBUTE_POSITION,
            ATTRIBUTE_OPTIONS,
        )
    };
    #[cfg(target_os = "linux")]
    let count = unsafe {
        libc::fgetxattr(
            directory.descriptor(),
            ATTRIBUTE.as_ptr(),
            bytes.as_mut_ptr().cast(),
            bytes.len(),
        )
    };
    if count.is_negative() {
        return Err(std::io::Error::last_os_error()).context(ATTRIBUTE_ERROR);
    }
    bytes.truncate(usize::try_from(count)?);
    Ok(bytes)
}

pub(super) fn write(directory: &PrivateDirectory, bytes: &[u8]) -> Result<()> {
    directory.verify_path()?;
    #[cfg(target_os = "macos")]
    let result = unsafe {
        libc::fsetxattr(
            directory.descriptor(),
            ATTRIBUTE.as_ptr(),
            bytes.as_ptr().cast(),
            bytes.len(),
            ATTRIBUTE_POSITION,
            ATTRIBUTE_OPTIONS,
        )
    };
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::fsetxattr(
            directory.descriptor(),
            ATTRIBUTE.as_ptr(),
            bytes.as_ptr().cast(),
            bytes.len(),
            ATTRIBUTE_OPTIONS,
        )
    };
    if result != libc::c_int::default() {
        return Err(std::io::Error::last_os_error()).context(ATTRIBUTE_ERROR);
    }
    directory.sync()
}
