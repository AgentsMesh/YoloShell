use super::constants::{
    MAX_CATALOG_ARTIFACT_BYTES, SQLITE_JOURNAL_SUFFIX, SQLITE_SHM_SUFFIX, SQLITE_WAL_SUFFIX,
};
use super::retained_artifact::RetainedArtifact;
use crate::paths::identity::FileIdentity;
use crate::paths::private_file::PrivateFileEntry;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const MAIN_ARTIFACT_INDEX: usize = 0;
const JOURNAL_ARTIFACT_INDEX: usize = 1;
const WAL_ARTIFACT_INDEX: usize = 2;
const SHM_ARTIFACT_INDEX: usize = 3;
const RETAINED_REPAIR_ARTIFACT_INDEX_ERROR: &str = "retained repair artifact index is invalid";

#[derive(Clone)]
pub(super) struct RetainedCatalogNamespace(Arc<NamespaceInner>);

pub(super) struct RetainedSnapshot(Vec<Option<FileIdentity>>);

pub(super) struct NamespaceInner {
    alias_shard: Arc<super::catalog_alias_lease::CatalogAliasShard>,
    path: PathBuf,
    entries: Vec<RetainedArtifact>,
}

impl RetainedCatalogNamespace {
    pub(super) fn register(main: PrivateFileEntry) -> Result<Self> {
        if let Some(retained) = super::retained_registry::acquire(&main)? {
            return Ok(Self(retained));
        }
        let alias_shard = Arc::new(super::catalog_alias_lease::CatalogAliasShard::retain(
            &main,
        )?);
        let entries = super::retained_registry_identity::candidate_entries(&main)?
            .into_iter()
            .map(|entry| RetainedArtifact::new(entry, Arc::clone(&alias_shard)))
            .collect();
        let path = main.path().to_path_buf();
        let inner = Arc::new(NamespaceInner {
            alias_shard,
            path,
            entries,
        });
        Ok(Self(super::retained_registry::register(inner)?))
    }

    pub(super) fn path(&self) -> &Path {
        &self.0.path
    }

    pub(super) fn acquire(main: &PrivateFileEntry) -> Result<Option<Self>> {
        super::retained_registry::acquire(main).map(|namespace| namespace.map(Self))
    }

    pub(super) fn cleanup_aliases(&self) -> Result<()> {
        super::secure_vfs_cleanup::cleanup_stale_aliases_in(self.0.alias_shard.directory())
    }

    #[cfg(test)]
    pub(super) fn alias_parent(&self) -> &Path {
        self.0.alias_shard.directory().path()
    }

    pub(super) fn snapshot(&self) -> Result<RetainedSnapshot> {
        let identities = self
            .0
            .entries
            .iter()
            .map(|entry| {
                let inspected =
                    entry
                        .entry()
                        .inspect(MAX_CATALOG_ARTIFACT_BYTES)
                        .map_err(|error| {
                            super::rebuild_reason::unsafe_artifact(format!(
                                "unsafe retained catalog artifact {}: {error:#}",
                                entry.path().display()
                            ))
                        })?;
                if inspected.is_some() {
                    let file = super::secure_vfs_retained_file::reopen(entry.entry())?;
                    entry.bind(super::secure_vfs_identity::FileIdentity::inspect(
                        &file.metadata()?,
                        entry.path(),
                    )?)?;
                }
                Ok(inspected)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(RetainedSnapshot(identities))
    }

    pub(super) fn revalidate(&self, snapshot: &RetainedSnapshot) -> Result<()> {
        for (index, (entry, expected)) in self.0.entries.iter().zip(&snapshot.0).enumerate() {
            let current = entry.entry().inspect(MAX_CATALOG_ARTIFACT_BYTES)?;
            let valid = match (*expected, current) {
                (None, Some(_)) if index == MAIN_ARTIFACT_INDEX => true,
                (Some(expected), Some(current)) => current.same_object(expected),
                (None, None) => true,
                _ => false,
            };
            anyhow::ensure!(
                valid,
                "retained catalog artifact changed while opening: {}",
                entry.path().display()
            );
        }
        Ok(())
    }

    pub(super) fn quarantine(&self) -> Result<()> {
        for entry in &self.0.entries {
            super::retained_quarantine::quarantine_entry(entry.entry())?;
            entry.clear_missing();
        }
        Ok(())
    }
}

impl Drop for NamespaceInner {
    fn drop(&mut self) {
        super::retained_registry::unregister(self);
    }
}

impl NamespaceInner {
    pub(super) fn entries(&self) -> &[RetainedArtifact] {
        &self.entries
    }
}

#[cfg(test)]
impl RetainedCatalogNamespace {
    pub(super) fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

pub(super) fn resolve(path: &Path) -> Option<RetainedArtifact> {
    super::retained_registry::resolve(path)
}

pub(super) fn is_registered(path: &Path) -> bool {
    super::retained_registry::is_registered(path)
}

include!("retained_namespace_repair.rs");
