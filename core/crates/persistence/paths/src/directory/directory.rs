use super::constants::*;
use super::directory_identity::DirectoryIdentity;
use anyhow::{Context, Result};
use std::ffi::{CStr, CString, OsStr};
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

#[path = "directory_empty_remove.rs"]
mod directory_empty_remove;
#[path = "directory_fast.rs"]
mod directory_fast;
#[cfg(any(test, feature = "persistence-test-support"))]
pub(crate) use directory_empty_remove::set_before_remove_hook as set_before_empty_remove_hook;
#[path = "directory_relative.rs"]
mod directory_relative;
#[path = "directory_revision.rs"]
mod directory_revision;
#[path = "directory_strict_create.rs"]
mod directory_strict_create;
#[path = "directory_sync.rs"]
pub(super) mod directory_sync;
#[cfg(any(test, feature = "persistence-test-support"))]
pub(crate) use directory_sync::fail_next as fail_next_sync;
#[path = "directory_verification.rs"]
mod directory_verification;
pub use directory_revision::DirectoryRevision;
pub use directory_verification::RetainedPathVerification;
pub use directory_verification::raw_errno_proves_path_identity_loss;

#[derive(Debug)]
pub struct PrivateDirectory {
    file: File,
    identity: DirectoryIdentity,
    path: PathBuf,
}

impl PrivateDirectory {
    pub fn ensure(path: &Path) -> Result<Self> {
        super::validation::validate_absolute(path)?;
        let resolved = super::platform::platform_path(path);
        let mut current = open_root()?;
        let mut current_path = PathBuf::from(std::path::MAIN_SEPARATOR_STR);
        let components = normal_components(&resolved)?;
        anyhow::ensure!(!components.is_empty(), "private directory cannot be root");
        let mut components = components.into_iter().peekable();
        while let Some(component) = components.next() {
            verify_descriptor_path(&current, &current_path)?;
            let final_component = components.peek().is_none();
            let name = entry_c_string(component)?;
            let (next, created) = open_or_create(&current, &name, &current_path)?;
            current_path.push(component);
            if created || final_component {
                verify_normalizable_private(&next, &current_path)?;
                set_private_mode(&next, &current_path)?;
            }
            if final_component {
                crate::private_acl::require_empty(&next, &current_path)?;
                DirectoryIdentity::inspect_private(&next.metadata()?, &current_path)?;
            } else {
                crate::private_acl::require_no_grants(&next, &current_path)?;
                DirectoryIdentity::inspect_ancestor(&next.metadata()?, &current_path)?;
            }
            current = next;
        }
        #[cfg(all(test, feature = "yolo-unit-tests"))]
        tests::before_ensure_reopen();
        let reopened = Self::open(path)?;
        anyhow::ensure!(
            reopened.identity == DirectoryIdentity::inspect_private(&current.metadata()?, path)?,
            "private directory changed while opening: {}",
            path.display()
        );
        Ok(reopened)
    }

    pub fn open(path: &Path) -> Result<Self> {
        super::validation::validate_absolute(path)?;
        let first = open_chain(&super::platform::platform_path(path))?;
        crate::private_acl::require_empty(&first, path)?;
        let identity = DirectoryIdentity::inspect_private(&first.metadata()?, path)?;
        #[cfg(all(test, feature = "yolo-unit-tests"))]
        tests::after_first_open();
        let second = open_chain(&super::platform::platform_path(path))?;
        crate::private_acl::require_empty(&second, path)?;
        anyhow::ensure!(
            identity == DirectoryIdentity::inspect_private(&second.metadata()?, path)?,
            "private directory changed while opening: {}",
            path.display()
        );
        Ok(Self {
            file: second,
            identity,
            path: path.to_path_buf(),
        })
    }

    pub fn descriptor(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}

pub(super) fn entry_c_string(name: &OsStr) -> Result<CString> {
    CString::new(name.as_bytes()).context("durable entry name contains a null byte")
}

fn open_root() -> Result<File> {
    open_directory(libc::AT_FDCWD, root_c_string()?.as_c_str()).context("open filesystem root")
}

fn open_chain(path: &Path) -> Result<File> {
    let mut current = open_root()?;
    let mut current_path = PathBuf::from(std::path::MAIN_SEPARATOR_STR);
    for component in normal_components(path)? {
        let name = entry_c_string(component)?;
        current = open_directory(current.as_raw_fd(), &name).with_context(|| {
            format!("open directory {}", current_path.join(component).display())
        })?;
        current_path.push(component);
        crate::private_acl::require_no_grants(&current, &current_path)?;
        DirectoryIdentity::inspect_ancestor(&current.metadata()?, &current_path)?;
    }
    Ok(current)
}

include!("directory_create.rs");

fn open_directory(parent: RawFd, name: &CStr) -> std::io::Result<File> {
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    let descriptor = unsafe { libc::openat(parent, name.as_ptr(), flags) };
    if descriptor < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }
}

fn set_private_mode(file: &File, path: &Path) -> Result<()> {
    let mode = libc::mode_t::try_from(PRIVATE_DIRECTORY_MODE)?;
    let result = unsafe { libc::fchmod(file.as_raw_fd(), mode) };
    if result != 0 {
        return Err(std::io::Error::last_os_error())
            .with_context(|| format!("secure private directory {}", path.display()));
    }
    crate::private_acl::clear(file, path)?;
    DirectoryIdentity::inspect_private(&file.metadata()?, path).map(|_| ())
}

fn verify_normalizable_private(file: &File, path: &Path) -> Result<()> {
    let metadata = file.metadata()?;
    anyhow::ensure!(
        metadata.uid() == unsafe { libc::geteuid() },
        "private directory owner mismatch: {}",
        path.display()
    );
    DirectoryIdentity::inspect_ancestor(&metadata, path).map(|_| ())
}

fn verify_descriptor_path(file: &File, path: &Path) -> Result<()> {
    let expected = DirectoryIdentity::inspect_ancestor(&file.metadata()?, path)?;
    let reopened = open_chain(path)?;
    let actual = DirectoryIdentity::inspect_ancestor(&reopened.metadata()?, path)?;
    anyhow::ensure!(
        expected == actual,
        "directory path changed: {}",
        path.display()
    );
    Ok(())
}

fn normal_components(path: &Path) -> Result<Vec<&OsStr>> {
    path.components()
        .skip(1)
        .map(|component| match component {
            Component::Normal(name) => Ok(name),
            _ => anyhow::bail!("path is not lexically normalized: {}", path.display()),
        })
        .collect()
}

fn root_c_string() -> Result<CString> {
    CString::new(std::path::MAIN_SEPARATOR_STR).context("filesystem root contains a null byte")
}
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/directory/directory_branch_tests.rs"]
mod tests;
