use anyhow::{Context, Result};
use std::ffi::{CStr, CString, OsString};
use std::mem::MaybeUninit;
use std::os::fd::RawFd;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::Path;

const QUARANTINE_MARKER: &str = ".quarantine-";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ArtifactIdentity {
    device: libc::dev_t,
    inode: libc::ino_t,
    kind: libc::mode_t,
    links: libc::nlink_t,
}

pub(super) fn quarantine_catalog(path: &Path) -> Result<()> {
    for artifact in super::artifacts::catalog_artifacts(path) {
        quarantine_artifact(&artifact, || Ok(()))?;
    }
    Ok(())
}

fn quarantine_artifact<F>(path: &Path, hook: F) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    let directory = crate::paths::secure_private_parent(path)?;
    let source = entry_name(path)?;
    let Some(identity) = inspect_optional(directory.descriptor(), &source, path)? else {
        return Ok(());
    };
    hook()?;
    anyhow::ensure!(
        inspect_required(directory.descriptor(), &source, path)? == identity,
        "catalog artifact changed before quarantine: {}",
        path.display()
    );
    let target = quarantine_name(path)?;
    rename_exclusive(directory.descriptor(), &source, &target)?;
    directory.sync()?;
    anyhow::ensure!(
        inspect_required(directory.descriptor(), &target, path)? == identity,
        "quarantined catalog artifact identity changed: {}",
        path.display()
    );
    directory.verify_path()
}

fn inspect_optional(parent: RawFd, name: &CStr, path: &Path) -> Result<Option<ArtifactIdentity>> {
    let mut metadata = MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            parent,
            name.as_ptr(),
            metadata.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == 0 {
        let metadata = unsafe { metadata.assume_init() };
        return Ok(Some(ArtifactIdentity {
            device: metadata.st_dev,
            inode: metadata.st_ino,
            kind: metadata.st_mode & libc::S_IFMT,
            links: metadata.st_nlink,
        }));
    }
    let error = std::io::Error::last_os_error();
    if error.kind() == std::io::ErrorKind::NotFound {
        Ok(None)
    } else {
        Err(error).with_context(|| format!("inspect catalog artifact {}", path.display()))
    }
}

fn inspect_required(parent: RawFd, name: &CStr, path: &Path) -> Result<ArtifactIdentity> {
    inspect_optional(parent, name, path)?
        .with_context(|| format!("catalog artifact disappeared: {}", path.display()))
}

fn rename_exclusive(parent: RawFd, source: &CStr, target: &CStr) -> Result<()> {
    #[cfg(target_os = "macos")]
    let result = unsafe {
        libc::renameatx_np(
            parent,
            source.as_ptr(),
            parent,
            target.as_ptr(),
            libc::RENAME_EXCL,
        )
    };
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::renameat2(
            parent,
            source.as_ptr(),
            parent,
            target.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let result = libc::ENOTSUP;
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    if result == libc::ENOTSUP {
        return Err(std::io::Error::from_raw_os_error(libc::ENOTSUP))
            .context("exclusive catalog quarantine rename is unsupported");
    }
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error()).context("quarantine catalog artifact")
    }
}

fn entry_name(path: &Path) -> Result<CString> {
    let name = path
        .file_name()
        .context("catalog artifact has no file name")?;
    CString::new(name.as_bytes()).context("catalog artifact name contains a null byte")
}

fn quarantine_name(path: &Path) -> Result<CString> {
    let mut name = OsString::from_vec(
        path.file_name()
            .context("catalog artifact has no file name")?
            .as_bytes()
            .to_vec(),
    );
    name.push(QUARANTINE_MARKER);
    name.push(uuid::Uuid::new_v4().to_string());
    CString::new(name.as_bytes()).context("catalog quarantine name contains a null byte")
}

#[cfg(test)]
pub(super) fn quarantine_with_hook<F>(path: &Path, hook: F) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    quarantine_artifact(path, hook)
}
