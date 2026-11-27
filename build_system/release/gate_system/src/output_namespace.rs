use anyhow::{Context, Result, ensure};
use std::collections::BTreeSet;
use std::ffi::CStr;
use yolo_release_contracts::{OUTPUT_ENTRIES_ERROR, OUTPUT_FILE_NAMES};
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_filesystem::constants::{
    CURRENT_DIRECTORY_COMPONENT, DIRECTORY_STREAM_END, PARENT_DIRECTORY_COMPONENT,
};

pub fn validate_exact_entries(directory: &DirectoryHandle) -> Result<()> {
    validate_named_entries(directory, OUTPUT_FILE_NAMES, OUTPUT_ENTRIES_ERROR)
}

pub fn validate_named_entries(
    directory: &DirectoryHandle,
    expected_names: &[&str],
    mismatch_error: &str,
) -> Result<()> {
    let entries = entry_names(directory)?;
    let expected = expected_names
        .iter()
        .map(|name| name.as_bytes().to_vec())
        .collect::<BTreeSet<_>>();
    ensure!(
        entries.into_iter().collect::<BTreeSet<_>>() == expected,
        mismatch_error.to_owned()
    );
    directory.verify_self()
}

pub fn entry_names(directory: &DirectoryHandle) -> Result<Vec<Vec<u8>>> {
    let descriptor = unsafe { libc::dup(directory.raw_fd()) };
    if descriptor < 0 {
        return Err(std::io::Error::last_os_error()).context("duplicate candidate directory");
    }
    let stream = unsafe { libc::fdopendir(descriptor) };
    if stream.is_null() {
        unsafe { libc::close(descriptor) };
        return Err(std::io::Error::last_os_error()).context("open candidate directory stream");
    }
    let stream = DirectoryStream(stream);
    unsafe { libc::rewinddir(stream.0) };
    let mut entries = Vec::new();
    loop {
        unsafe { *libc::__error() = DIRECTORY_STREAM_END };
        let entry = unsafe { libc::readdir(stream.0) };
        if entry.is_null() {
            let error = std::io::Error::last_os_error();
            ensure!(
                error.raw_os_error() == Some(DIRECTORY_STREAM_END),
                "read candidate directory: {error}"
            );
            break;
        }
        let bytes = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if bytes != CURRENT_DIRECTORY_COMPONENT.as_bytes()
            && bytes != PARENT_DIRECTORY_COMPONENT.as_bytes()
        {
            entries.push(bytes.to_vec());
        }
    }
    entries.sort();
    directory.verify_self()?;
    Ok(entries)
}

struct DirectoryStream(*mut libc::DIR);

impl Drop for DirectoryStream {
    fn drop(&mut self) {
        unsafe { libc::closedir(self.0) };
    }
}
