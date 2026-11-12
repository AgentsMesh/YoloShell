use super::retained_artifact::RetainedArtifact;
use crate::paths::private_file::PrivateFileEntry;
use anyhow::{Context, Result};
use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::os::fd::RawFd;

const CASE_SENSITIVITY_CONTEXT: &str = "query retained catalog volume case sensitivity";
const MAIN_ARTIFACT_MISSING: &str = "retained catalog has no main artifact";
const PARENT_INSPECTION_CONTEXT: &str = "inspect retained catalog parent";
const PHYSICAL_NAMESPACE_CONFLICT: &str =
    "catalog retained path belongs to another physical namespace";

pub(super) fn same_slot(left: &[RetainedArtifact], right: &[RetainedArtifact]) -> Result<bool> {
    let left = left.first().context(MAIN_ARTIFACT_MISSING)?;
    let right = right.first().context(MAIN_ARTIFACT_MISSING)?;
    same_entry_slot(left.entry(), right.entry())
}

pub(super) fn ensure_entries_disjoint(
    left: &[RetainedArtifact],
    right: &[RetainedArtifact],
) -> Result<()> {
    for candidate in left {
        ensure_candidate_disjoint(std::slice::from_ref(candidate.entry()), right)?;
    }
    Ok(())
}

pub(super) fn ensure_candidate_disjoint(
    candidates: &[PrivateFileEntry],
    registered: &[RetainedArtifact],
) -> Result<()> {
    for candidate in candidates {
        for existing in registered {
            anyhow::ensure!(
                candidate.path() != existing.path()
                    && !same_entry_slot(candidate, existing.entry())?,
                "{PHYSICAL_NAMESPACE_CONFLICT}: {}",
                candidate.path().display()
            );
        }
    }
    Ok(())
}

pub(super) fn candidate_entries(main: &PrivateFileEntry) -> Result<Vec<PrivateFileEntry>> {
    Ok(vec![
        main.reopen()?,
        main.sibling_with_suffix(super::constants::SQLITE_JOURNAL_SUFFIX)?,
        main.sibling_with_suffix(super::constants::SQLITE_WAL_SUFFIX)?,
        main.sibling_with_suffix(super::constants::SQLITE_SHM_SUFFIX)?,
    ])
}

pub(super) fn same_entry_slot(left: &PrivateFileEntry, right: &PrivateFileEntry) -> Result<bool> {
    Ok(descriptor_identity(left.parent_descriptor())?
        == descriptor_identity(right.parent_descriptor())?
        && names_equal(
            left.parent_descriptor(),
            left.file_name(),
            right.file_name(),
        )?)
}

fn descriptor_identity(descriptor: RawFd) -> Result<(libc::dev_t, libc::ino_t)> {
    let mut metadata = MaybeUninit::<libc::stat>::uninit();
    let result = unsafe { libc::fstat(descriptor, metadata.as_mut_ptr()) };
    if result != 0 {
        return Err(std::io::Error::last_os_error()).context(PARENT_INSPECTION_CONTEXT);
    }
    let metadata = unsafe { metadata.assume_init() };
    Ok((metadata.st_dev, metadata.st_ino))
}

fn names_equal(parent: RawFd, left: &CStr, right: &CStr) -> Result<bool> {
    if left == right {
        return Ok(true);
    }
    #[cfg(target_os = "macos")]
    {
        let case_sensitive = unsafe { libc::fpathconf(parent, libc::_PC_CASE_SENSITIVE) };
        anyhow::ensure!(
            case_sensitive == 0 || case_sensitive == 1,
            CASE_SENSITIVITY_CONTEXT
        );
        if case_sensitive == 0 {
            return Ok(left.to_bytes().eq_ignore_ascii_case(right.to_bytes()));
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = parent;
    Ok(false)
}
