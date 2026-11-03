use super::secure_vfs_identity::FileIdentity;
use crate::paths::private_file::PrivateFileEntry;
use anyhow::{Context, Result};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, Weak};

pub(super) struct RetainedEntry {
    alias_shard: Arc<super::catalog_alias_lease::CatalogAliasShard>,
    entry: PrivateFileEntry,
    logical_path: std::path::PathBuf,
    expected: Mutex<Option<FileIdentity>>,
}

#[derive(Clone)]
pub(super) struct RetainedArtifact(Arc<RetainedEntry>);

impl RetainedArtifact {
    pub(super) fn new(
        entry: PrivateFileEntry,
        alias_shard: Arc<super::catalog_alias_lease::CatalogAliasShard>,
    ) -> Self {
        let logical_path = entry.path().to_path_buf();
        Self::new_at_path(entry, alias_shard, logical_path)
    }

    pub(super) fn new_at_path(
        entry: PrivateFileEntry,
        alias_shard: Arc<super::catalog_alias_lease::CatalogAliasShard>,
        logical_path: std::path::PathBuf,
    ) -> Self {
        Self(Arc::new(RetainedEntry {
            alias_shard,
            entry,
            logical_path,
            expected: Mutex::new(None),
        }))
    }

    pub(super) fn from_weak(entry: &Weak<RetainedEntry>) -> Option<Self> {
        entry.upgrade().map(Self)
    }

    pub(super) fn downgrade(&self) -> Weak<RetainedEntry> {
        Arc::downgrade(&self.0)
    }

    pub(super) fn ptr_eq(&self, entry: &Weak<RetainedEntry>) -> bool {
        entry.ptr_eq(&self.downgrade())
    }

    pub(super) fn entry(&self) -> &PrivateFileEntry {
        &self.0.entry
    }

    pub(super) fn path(&self) -> &Path {
        &self.0.logical_path
    }

    pub(super) fn alias_parent(&self) -> Result<crate::paths::directory::PrivateDirectory> {
        self.0.alias_shard.directory().try_clone()
    }

    pub(super) fn bind(&self, identity: FileIdentity) -> Result<()> {
        let mut expected = self.lock_expected();
        match *expected {
            None => *expected = Some(identity),
            Some(current) if current == identity => {}
            Some(_) => {
                return Err(super::secure_vfs_identity::unsafe_artifact(format!(
                    "retained SQLite artifact identity changed: {}",
                    self.path().display()
                )));
            }
        }
        Ok(())
    }

    pub(super) fn verify(&self, identity: FileIdentity) -> Result<()> {
        let expected = self
            .expected()
            .context("retained SQLite artifact has no expected identity")?;
        anyhow::ensure!(
            expected == identity,
            "retained SQLite artifact identity mismatch"
        );
        Ok(())
    }

    pub(super) fn expected(&self) -> Option<FileIdentity> {
        *self.lock_expected()
    }

    pub(super) fn clear(&self, identity: FileIdentity) -> Result<()> {
        let mut expected = self.lock_expected();
        anyhow::ensure!(
            expected.is_none() || *expected == Some(identity),
            "retained SQLite artifact identity changed before clearing"
        );
        *expected = None;
        Ok(())
    }

    pub(super) fn clear_missing(&self) {
        *self.lock_expected() = None;
    }

    fn lock_expected(&self) -> MutexGuard<'_, Option<FileIdentity>> {
        self.0
            .expected
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
