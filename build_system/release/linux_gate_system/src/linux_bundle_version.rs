use anyhow::{Context, Result, ensure};
use std::{fs::OpenOptions, io::Read, os::unix::fs::OpenOptionsExt, path::Path};
use yolo_distribution_bundle::ArtifactRole;
use yolo_release_contracts::{LINUX_BUNDLE_VERSION_SUFFIX, LINUX_RUNTIME_VERSION};
pub(crate) const VERSION_TYPE_ERROR: &str = "Linux runtime version must be a regular file";
pub(crate) const VERSION_SIZE_ERROR: &str = "Linux runtime version size is invalid";
const OPEN_CONTEXT: &str = "open Linux runtime version";
const VERSION_MISMATCH_ERROR: &str = "Linux runtime version mismatch";
const SENTINEL_BYTES: u64 = 1;

pub(super) fn verify(directory: &Path, role: ArtifactRole) -> Result<()> {
    let path = directory.join(format!(
        "{}{LINUX_BUNDLE_VERSION_SUFFIX}",
        role.executable_name()
    ));
    let expected = format!("{} {LINUX_RUNTIME_VERSION}\n", role.executable_name());
    let expected_length = u64::try_from(expected.len()).context(VERSION_SIZE_ERROR)?;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .context(OPEN_CONTEXT)?;
    let metadata = file.metadata()?;
    ensure!(metadata.is_file(), VERSION_TYPE_ERROR);
    ensure!(metadata.len() <= expected_length, VERSION_SIZE_ERROR);
    let mut bytes = Vec::with_capacity(expected.len());
    file.take(expected_length.saturating_add(SENTINEL_BYTES))
        .read_to_end(&mut bytes)?;
    ensure!(bytes == expected.as_bytes(), VERSION_MISMATCH_ERROR);
    Ok(())
}
