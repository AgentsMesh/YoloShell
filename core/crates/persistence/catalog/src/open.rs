use super::constants::*;
use super::*;
use anyhow::Context;
use rusqlite::{OpenFlags, TransactionBehavior, params};
use std::path::{Path, PathBuf};

pub struct CatalogRepairEntry {
    entry: crate::paths::private_file::PrivateFileEntry,
    expected: crate::paths::identity::FileIdentity,
}

impl Catalog {
    pub fn open(path: &Path) -> Result<Self> {
        let parent = crate::paths::secure_private_parent(path)?;
        let entry = crate::paths::private_file::PrivateFileEntry::from_directory(parent, path)?;
        if let Some(retained) = retained_namespace::RetainedCatalogNamespace::acquire(&entry)? {
            let retained_path = retained.path().to_path_buf();
            return Self::open_with_namespace(&retained_path, Some(retained));
        }
        Self::open_with_namespace(path, None)
    }

    pub(super) fn open_with_namespace(
        path: &Path,
        retained: Option<retained_namespace::RetainedCatalogNamespace>,
    ) -> Result<Self> {
        let snapshot = match &retained {
            Some(namespace) => {
                namespace.cleanup_aliases()?;
                OpenSnapshot::Retained(namespace.snapshot()?)
            }
            None => {
                crate::paths::ensure_private_parent(path)?;
                secure_vfs_cleanup::cleanup_stale_aliases(path)?;
                OpenSnapshot::Path(artifacts::validate_catalog_path(path)?)
            }
        };
        let sqlite_path = match &retained {
            Some(namespace) => namespace.path().to_path_buf(),
            None => artifacts::sqlite_catalog_path(path)?,
        };
        let flags = OpenFlags::default() | OpenFlags::SQLITE_OPEN_NOFOLLOW;
        let mut connection = secure_vfs::open(&sqlite_path, flags)
            .with_context(|| format!("open catalog {}", path.display()))?;
        match (&retained, &snapshot) {
            (Some(namespace), OpenSnapshot::Retained(snapshot)) => {
                namespace.revalidate(snapshot)?
            }
            (None, OpenSnapshot::Path(snapshot)) => artifacts::revalidate_catalog_path(snapshot)?,
            _ => unreachable!("catalog snapshot source must match its namespace"),
        }
        connection.busy_timeout(SQLITE_BUSY_TIMEOUT)?;
        secure_vfs::sqlite_result(connection.pragma_update(
            None,
            JOURNAL_MODE_PRAGMA,
            JOURNAL_MODE_VALUE,
        ))?;
        secure_vfs::sqlite_result(connection.pragma_update(
            None,
            SYNCHRONOUS_PRAGMA,
            SYNCHRONOUS_VALUE,
        ))?;
        secure_vfs::sqlite_result(connection.pragma_update(
            None,
            FOREIGN_KEYS_PRAGMA,
            FOREIGN_KEYS_VALUE,
        ))?;
        let schema = catalog_schema_sql();
        let transaction = rebuild_reason::schema_result(
            connection.transaction_with_behavior(TransactionBehavior::Immediate),
        )?;
        rebuild_reason::schema_result(transaction.execute_batch(&schema))?;
        rebuild_reason::schema_result(transaction.execute(
            INSERT_METADATA_SQL,
            params![SCHEMA_VERSION_KEY, SCHEMA_VERSION.to_string()],
        ))?;
        rebuild_reason::schema_result(transaction.execute(
            INSERT_METADATA_SQL,
            params![
                CONTROLLER_GENERATION_KEY,
                INITIAL_CONTROLLER_GENERATION.to_string()
            ],
        ))?;
        rebuild_reason::schema_result(transaction.commit())?;
        let actual: String = rebuild_reason::schema_result(connection.query_row(
            READ_METADATA_SQL,
            [SCHEMA_VERSION_KEY],
            |row| row.get(METADATA_VALUE_COLUMN),
        ))?;
        if actual != SCHEMA_VERSION.to_string() {
            return Err(rebuild_reason::schema(actual));
        }
        let catalog = Self {
            connection,
            _retained: retained,
        };
        catalog.controller_generation()?;
        Ok(catalog)
    }

    pub fn open_repair_entry(captured: CatalogRepairEntry, logical_path: PathBuf) -> Result<Self> {
        let CatalogRepairEntry { entry, expected } = captured;
        let namespace = retained_namespace::RetainedCatalogNamespace::register_repair(
            entry,
            logical_path,
            expected,
        )?;
        let verification = namespace.clone();
        let path = namespace.path().to_path_buf();
        let catalog = Self::open_with_namespace(&path, Some(namespace))?;
        verification.require_main_object(expected)?;
        catalog.integrity_check()?;
        Ok(catalog)
    }

    pub fn capture_repair_entry(
        entry: crate::paths::private_file::PrivateFileEntry,
    ) -> Result<CatalogRepairEntry> {
        let expected = entry
            .inspect(constants::MAX_CATALOG_ARTIFACT_BYTES)?
            .context("catalog repair target is missing")?;
        Ok(CatalogRepairEntry { entry, expected })
    }
}

enum OpenSnapshot {
    Path(artifacts::CatalogPathSnapshot),
    Retained(retained_namespace::RetainedSnapshot),
}
