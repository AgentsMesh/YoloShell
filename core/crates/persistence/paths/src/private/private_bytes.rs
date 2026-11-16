use super::constants::{ATOMIC_FILE_PREFIX, ATOMIC_FILE_SUFFIX};
use super::private_file::{PrivateFileEntry, TemporaryPrivateFile};
use anyhow::{Context, Result};
use std::io::{Read, Write};

const BYTE_LIMIT_ERROR: &str = "private bytes exceed limit";
const INSTALLED_BYTES_CHANGED_ERROR: &str = "installed private bytes changed";
const READ_LIMIT_SENTINEL_BYTES: u64 = 1;
const STAGING_CLEANUP_CONTEXT: &str = "clean private byte staging file after write failure";

pub fn write_private_bytes_guarded<F>(
    entry: PrivateFileEntry,
    bytes: &[u8],
    max_bytes: u64,
    guard: F,
) -> Result<()>
where
    F: Fn() -> Result<()>,
{
    write_private_bytes_guarded_with_commit_boundary(entry, bytes, max_bytes, guard, || Ok(()))
}

fn write_private_bytes_guarded_with_commit_boundary<F, G>(
    entry: PrivateFileEntry,
    bytes: &[u8],
    max_bytes: u64,
    guard: F,
    mut commit_boundary: G,
) -> Result<()>
where
    F: Fn() -> Result<()>,
    G: FnMut() -> Result<()>,
{
    anyhow::ensure!(u64::try_from(bytes.len())? <= max_bytes, BYTE_LIMIT_ERROR);
    guard()?;
    let mut temporary = entry.temporary(ATOMIC_FILE_PREFIX, ATOMIC_FILE_SUFFIX)?;
    let result = install(
        &entry,
        &mut temporary,
        bytes,
        max_bytes,
        &guard,
        &mut commit_boundary,
    );
    if let Err(error) = result {
        entry
            .remove_temporary(&temporary)
            .context(STAGING_CLEANUP_CONTEXT)?;
        return Err(error);
    }
    Ok(())
}

fn install<F, G>(
    entry: &PrivateFileEntry,
    temporary: &mut TemporaryPrivateFile,
    bytes: &[u8],
    max_bytes: u64,
    guard: &F,
    commit_boundary: &mut G,
) -> Result<()>
where
    F: Fn() -> Result<()>,
    G: FnMut() -> Result<()>,
{
    temporary.file.write_all(bytes)?;
    temporary.sync_and_inspect(entry.path(), max_bytes)?;
    guard()?;
    entry.rename_temporary_with_fences(
        temporary,
        max_bytes,
        || guard(),
        || {
            commit_boundary()?;
            guard()
        },
        || {},
    )?;
    guard()?;
    entry.verify_current_content_version(temporary.identity, max_bytes)?;
    verify_installed(entry, bytes, max_bytes)
}

fn verify_installed(entry: &PrivateFileEntry, expected: &[u8], max_bytes: u64) -> Result<()> {
    let mut installed = entry.open_read(max_bytes)?;
    let mut actual = Vec::with_capacity(expected.len());
    Read::by_ref(&mut installed.file)
        .take(max_bytes.saturating_add(READ_LIMIT_SENTINEL_BYTES))
        .read_to_end(&mut actual)?;
    anyhow::ensure!(actual == expected, INSTALLED_BYTES_CHANGED_ERROR);
    installed.identity.verify_file_bounded(
        &installed.file,
        u64::try_from(actual.len())?,
        entry.path(),
        max_bytes,
    )?;
    entry.verify_current_unchanged(installed.identity, max_bytes)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/private/private_bytes_adversarial_tests.rs"]
mod adversarial_tests;

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../../unit_tests/private/private_bytes_inline_tests.rs"]
mod tests;
