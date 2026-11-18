use std::collections::BTreeMap;

use yolo_paths::coordination::CoordinationLock;
use yolo_paths::{atomic_write_json, read_json};
use yolo_trusted_plugin_manifest::{TrustedPluginManifest, load};

use crate::{RegistryPaths, RegistryRecord, source_digest};

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("plugin registry I/O failed")]
    Io(#[from] std::io::Error),
    #[error("plugin registry document is invalid")]
    Invalid,
    #[error("plugin manifest is invalid: {0}")]
    Manifest(#[from] yolo_trusted_plugin_manifest::ManifestError),
    #[error("plugin id is already linked")]
    Duplicate,
    #[error("plugin is not linked")]
    NotFound,
    #[error("plugin source changed since link")]
    DigestMismatch,
    #[error("plugin registry path is invalid")]
    Paths,
    #[error("plugin registry is busy")]
    Busy,
}

pub struct Registry {
    records: BTreeMap<String, RegistryRecord>,
    _lock: Option<CoordinationLock>,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema_version: u16,
    plugins: Vec<RegistryRecord>,
}

const DOCUMENT_SCHEMA: u16 = 1;

impl Registry {
    pub fn load(paths: &RegistryPaths) -> Result<Self, RegistryError> {
        Self::open(paths, false)
    }

    /// Serializes mutations across processes; hold it for the whole command.
    pub fn load_locked(paths: &RegistryPaths) -> Result<Self, RegistryError> {
        Self::open(paths, true)
    }

    fn open(paths: &RegistryPaths, locked: bool) -> Result<Self, RegistryError> {
        let lock = if locked {
            Some(crate::lock::acquire(paths)?)
        } else {
            paths.ensure().map_err(|_| RegistryError::Paths)?;
            None
        };
        let path = paths.registry_file();
        let exists = match std::fs::symlink_metadata(&path) {
            Ok(_) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(_) => return Err(RegistryError::Paths),
        };
        if !exists { return Ok(Self { records: BTreeMap::new(), _lock: lock }); }
        let document: Document = read_json(&path).map_err(|_| RegistryError::Invalid)?;
        if document.schema_version != DOCUMENT_SCHEMA {
            return Err(RegistryError::Invalid);
        }
        let mut records = BTreeMap::new();
        for record in document.plugins {
            if records.insert(record.id.clone(), record).is_some() {
                return Err(RegistryError::Invalid);
            }
        }
        Ok(Self {
            records,
            _lock: lock,
        })
    }

    pub fn save(&self, paths: &RegistryPaths) -> Result<(), RegistryError> {
        paths.ensure().map_err(|_| RegistryError::Paths)?;
        let document = Document {
            schema_version: DOCUMENT_SCHEMA,
            plugins: self.records.values().cloned().collect(),
        };
        atomic_write_json(&paths.registry_file(), &document).map_err(|_| RegistryError::Invalid)
    }

    pub fn records(&self) -> impl Iterator<Item = &RegistryRecord> {
        self.records.values()
    }

    pub fn get(&self, id: &str) -> Option<&RegistryRecord> {
        self.records.get(id)
    }

    pub(crate) fn insert(&mut self, record: RegistryRecord) {
        self.records.insert(record.id.clone(), record);
    }

    pub fn unlink(&mut self, id: &str, paths: &RegistryPaths) -> Result<(), RegistryError> {
        self.records.remove(id).ok_or(RegistryError::NotFound)?;
        self.save(paths)
    }

    pub fn set_enabled(
        &mut self,
        id: &str,
        enabled: bool,
        paths: &RegistryPaths,
    ) -> Result<(), RegistryError> {
        let record = self.records.get_mut(id).ok_or(RegistryError::NotFound)?;
        record.enabled = enabled;
        self.save(paths)
    }

    pub fn revalidate(
        &self,
        id: &str,
    ) -> Result<(&RegistryRecord, TrustedPluginManifest), RegistryError> {
        let record = self.records.get(id).ok_or(RegistryError::NotFound)?;
        let manifest = load(&record.source)?;
        let digest = source_digest(&record.source)?;
        if digest != record.digest || manifest.id != record.id {
            return Err(RegistryError::DigestMismatch);
        }
        Ok((record, manifest))
    }

    pub fn set_startup(
        &mut self,
        id: &str,
        status: crate::StartupStatus,
        paths: &RegistryPaths,
    ) -> Result<(), RegistryError> {
        let record = self.records.get_mut(id).ok_or(RegistryError::NotFound)?;
        record.last_startup = Some(status);
        self.save(paths)
    }
}
