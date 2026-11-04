#[path = "constants.rs"]
mod constants;
#[path = "verification.rs"]
mod verification;

pub use verification::NamespaceVerification;

use crate::paths::directory::PrivateDirectory;
use crate::paths::private_file::PrivateFileEntry;
use anyhow::{Context, Result};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use constants::*;

#[derive(Debug)]
struct NamespaceAnchor {
    label: &'static str,
    directory: PrivateDirectory,
}

#[derive(Debug)]
pub struct NamespaceLease {
    anchors: Vec<NamespaceAnchor>,
    lost: AtomicBool,
}

#[derive(Debug)]
pub struct NamespaceOwnershipLoss {
    _private: (),
}

impl NamespaceLease {
    pub fn capture(anchors: &[(&'static str, &Path)]) -> Result<Self> {
        let anchors = anchors
            .iter()
            .map(|(label, path)| {
                let path = crate::paths::platform::platform_path(path);
                PrivateDirectory::open(&path)
                    .with_context(|| format!("capture {label} namespace"))
                    .map(|directory| NamespaceAnchor { label, directory })
            })
            .collect::<Result<Vec<_>>>()?;
        Self::from_anchors(anchors)
    }

    pub fn from_directories(anchors: Vec<(&'static str, PrivateDirectory)>) -> Result<Self> {
        Self::from_anchors(
            anchors
                .into_iter()
                .map(|(label, directory)| NamespaceAnchor { label, directory })
                .collect(),
        )
    }

    fn from_anchors(anchors: Vec<NamespaceAnchor>) -> Result<Self> {
        let lease = Self {
            anchors,
            lost: AtomicBool::new(false),
        };
        lease.verify()?;
        Ok(lease)
    }

    pub fn directory(&self, path: &Path) -> Result<PrivateDirectory> {
        self.with_anchor(path, false)
    }

    pub fn ensure_directory(&self, path: &Path) -> Result<PrivateDirectory> {
        self.with_anchor(path, true)
    }

    pub fn file(&self, path: &Path) -> Result<PrivateFileEntry> {
        let path = crate::paths::platform::platform_path(path);
        let parent = path.parent().context("namespace file has no parent")?;
        PrivateFileEntry::from_directory(self.directory(parent)?, &path)
    }

    pub fn write_json<T: Serialize>(&self, path: &Path, value: &T) -> Result<()> {
        let entry = self.file(path)?;
        crate::paths::atomic_write_entry_guarded(entry, value, || self.verify())?;
        self.verify()
    }

    pub fn read_json<T: DeserializeOwned>(&self, path: &Path) -> Result<T> {
        let entry = self.file(path)?;
        let value = crate::paths::read_entry_guarded(entry, || self.verify())?;
        self.verify()?;
        Ok(value)
    }

    pub fn file_is_missing(&self, path: &Path) -> Result<bool> {
        let entry = self.file(path)?;
        let missing = entry.is_missing()?;
        self.verify()?;
        Ok(missing)
    }

    pub fn remove_file(&self, path: &Path) -> Result<()> {
        let entry = self.file(path)?;
        self.verify()?;
        entry.remove()?;
        self.verify()
    }

    pub fn remove_json_if_matches<T>(&self, path: &Path, expected: &T) -> Result<()>
    where
        T: DeserializeOwned + PartialEq,
    {
        self.remove_json_if_matches_with_boundary(path, expected, || Ok(()))
    }

    pub fn remove_json_if_matches_with_boundary<T, F>(
        &self,
        path: &Path,
        expected: &T,
        boundary: F,
    ) -> Result<()>
    where
        T: DeserializeOwned + PartialEq,
        F: FnOnce() -> Result<()>,
    {
        let entry = self.file(path)?;
        let (current, identity): (T, _) =
            crate::paths::read_entry_guarded_with_identity(entry, || self.verify())?;
        anyhow::ensure!(&current == expected, CONDITIONAL_REMOVE_VALUE_CHANGED_ERROR);
        boundary()?;
        self.verify()?;
        self.file(path)?.remove_if_unchanged(identity)?;
        self.verify()
    }

    fn with_anchor(&self, path: &Path, ensure: bool) -> Result<PrivateDirectory> {
        anyhow::ensure!(!self.lost.load(Ordering::Acquire), ALREADY_LOST_ERROR);
        let path = crate::paths::platform::platform_path(path);
        let anchor = self.anchor(&path)?;
        let directory = if ensure {
            anchor.directory.ensure_descendant(&path)?
        } else {
            anchor.directory.open_descendant(&path)?
        };
        self.verify()?;
        Ok(directory)
    }

    fn anchor(&self, path: &Path) -> Result<&NamespaceAnchor> {
        let path = crate::paths::platform::platform_path(path);
        let mut selected: Option<(&NamespaceAnchor, usize)> = None;
        for anchor in &self.anchors {
            if let Ok(relative) = path.strip_prefix(anchor.directory.path()) {
                let depth = relative.components().count();
                if selected.is_none_or(|(_, current)| depth < current) {
                    selected = Some((anchor, depth));
                }
            }
        }
        selected
            .map(|(anchor, _)| anchor)
            .with_context(|| format!("path is outside retained namespace: {}", path.display()))
    }

    #[cfg(test)]
    pub fn is_lost(&self) -> bool {
        self.lost.load(Ordering::Acquire)
    }
}

impl crate::paths::ResumeMarkerNamespace for NamespaceLease {
    fn resume_marker_file(&self, path: &Path) -> Result<PrivateFileEntry> {
        self.file(path)
    }

    fn verify_resume_marker_namespace(&self) -> Result<()> {
        self.verify()
    }
}

include!("write_commit.rs");
include!("read_versions.rs");
include!("write_serialized.rs");
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/namespace/tests.rs"]
mod tests;
