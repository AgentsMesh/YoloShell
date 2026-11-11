use crate::constants::{TEMPORARY_DIRECTORY_MODE, TEMPORARY_NO_FLAGS, TEMPORARY_SEALED_MODE};
use crate::temporary_retirement::{confirm_or_restore, random_name, rename, unlink};
use crate::{DirectoryHandle, DirectorySeal};
use anyhow::{Context, Result, ensure};
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use tempfile::Builder;

const CREATE_SUCCESS: libc::c_int = 0;
const PRESERVATION_CONTEXT: &str = "release temporary directory preserved";

#[derive(Debug)]
pub struct TemporaryDirectory {
    parent: DirectoryHandle,
    root: DirectoryHandle,
    resources: crate::DirectoryResourceOwner,
    name: CString,
    children: Vec<(String, DirectoryHandle)>,
    preserved: Vec<String>,
    cleanup_attempted: bool,
    release_seal: bool,
}

impl TemporaryDirectory {
    pub fn create(prefix: &str) -> Result<Self> {
        // Relinquish pathname cleanup before any fallible custody or permission work.
        let path = Builder::new().prefix(prefix).tempdir()?.keep();
        Self::capture(&path)
            .with_context(|| format!("{PRESERVATION_CONTEXT} at {}", path.display()))
    }

    fn capture(path: &Path) -> Result<Self> {
        let parent =
            DirectoryHandle::open(path.parent().context("temporary directory has no parent")?)?;
        let name = CString::new(
            path.file_name()
                .context("temporary directory has no name")?
                .as_bytes(),
        )?;
        let mut root = DirectoryHandle::open_at_bytes(&parent, name.as_bytes())?;
        let resources = crate::DirectoryResourceOwner::attach(&mut root);
        let owner = Self {
            parent,
            root,
            resources,
            name,
            children: Vec::new(),
            preserved: Vec::new(),
            cleanup_attempted: false,
            release_seal: false,
        };
        owner.root.set_mode(TEMPORARY_DIRECTORY_MODE)?;
        Ok(owner)
    }

    pub fn handle(&self) -> &DirectoryHandle {
        &self.root
    }

    pub fn path(&self) -> PathBuf {
        self.parent
            .display_path()
            .join(std::ffi::OsStr::from_bytes(self.name.as_bytes()))
    }

    pub fn create_child(&mut self, name: &str) -> Result<DirectoryHandle> {
        let result = self.capture_child(name);
        if let Err(error) = &result {
            self.preserve(format!("incomplete child {name}: {error:#}"));
        }
        result.with_context(|| format!("temporary child custody at {}", self.path().display()))
    }

    fn capture_child(&mut self, name: &str) -> Result<DirectoryHandle> {
        let component = CString::new(name)?;
        ensure!(
            unsafe {
                libc::mkdirat(
                    self.root.raw_fd(),
                    component.as_ptr(),
                    TEMPORARY_DIRECTORY_MODE as libc::mode_t,
                )
            } == CREATE_SUCCESS,
            "create retained temporary child: {}",
            std::io::Error::last_os_error()
        );
        let child = DirectoryHandle::open_at(&self.root, name)?;
        self.children.push((name.to_owned(), child.clone()));
        child.set_mode(TEMPORARY_DIRECTORY_MODE)?;
        Ok(child)
    }

    fn preserve(&mut self, reason: String) {
        self.preserved.push(reason);
    }

    pub fn retain_resource_use(&self, description: &str) -> Result<crate::DirectoryResourceUse> {
        self.root.retain_resource_use(description)
    }

    pub fn seal(&mut self) -> Result<DirectorySeal> {
        self.release_seal = true;
        self.root.set_mode(TEMPORARY_SEALED_MODE)?;
        self.root.set_flags(libc::UF_IMMUTABLE)?;
        self.root.sync_all()?;
        self.root.seal()
    }

    pub fn cleanup(&mut self) -> Result<()> {
        self.cleanup_with(|| Ok(()))
    }

    pub(super) fn cleanup_with(
        &mut self,
        before_retire: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        self.cleanup_attempted = true;
        ensure!(
            self.preserved.is_empty(),
            "{PRESERVATION_CONTEXT}: {}",
            self.preserved.join("; ")
        );
        self.resources.begin_retirement()?;
        self.parent.verify_path()?;
        self.parent
            .verify_child_bytes(self.name.as_bytes(), &self.root)?;
        for (name, expected) in &self.children {
            self.root.verify_child(name, expected)?;
        }
        if self.release_seal {
            self.root.set_flags(TEMPORARY_NO_FLAGS)?;
        }
        before_retire()?;
        let retired = random_name()?;
        let retired_path = self
            .parent
            .display_path()
            .join(std::ffi::OsStr::from_bytes(retired.as_bytes()));
        rename(&self.parent, &self.name, &retired)?;
        let confirmation = self
            .parent
            .verify_child_bytes(retired.as_bytes(), &self.root);
        confirm_or_restore(
            &self.parent,
            &self.name,
            &retired,
            &retired_path,
            confirmation,
        )?;
        self.name = retired;
        crate::temporary_cleanup::clear(&self.root, &self.path(), &self.children)?;
        unlink(&self.parent, &self.name, true)
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        if !self.cleanup_attempted {
            if let Err(error) = self.cleanup() {
                eprintln!(
                    "{PRESERVATION_CONTEXT} at {}: {error:#}",
                    self.path().display()
                );
            }
        }
    }
}
