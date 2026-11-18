use crate::{SetupRecord, constants::*, model::BoundRecord};
use anyhow::{Result, ensure};
use std::path::PathBuf;
use uuid::Uuid;
use yolo_paths::{
    directory::PrivateDirectory, identity::FileIdentity, namespace_lease::NamespaceLease,
    private_file::io_kind,
};

/// Read-only custody across external observations; never grants transition authority.
pub struct SetupObservation {
    namespace: NamespaceLease,
    path: PathBuf,
    record: BoundRecord,
    file: FileIdentity,
}

impl SetupObservation {
    pub fn observe(parent: &PrivateDirectory, id: Uuid) -> Result<Option<Self>> {
        let parent = crate::parent::retain(parent.try_clone()?)?;
        ensure!(!id.is_nil(), CONFLICT);
        parent.verify_path()?;
        let directory_path = parent.path().join(id.to_string());
        let directory = match parent.open_descendant(&directory_path) {
            Ok(directory) => directory,
            Err(error) if io_kind(&error) == Some(std::io::ErrorKind::NotFound) => {
                parent.verify_path()?;
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        let identity = directory.durable_identity()?;
        let path = directory.path().join(RECORD_FILE);
        let namespace = NamespaceLease::from_directories(vec![
            (PARENT, parent.try_clone()?),
            (ENTRY, directory),
        ])?;
        let (record, file): (BoundRecord, FileIdentity) =
            namespace.read_json_with_identity(&path)?;
        record.record.validate()?;
        ensure!(
            record.parent == parent.durable_identity()?
                && record.directory == identity
                && record.record.request.request.fields().request_id == id,
            CONFLICT
        );
        let observation = Self {
            namespace,
            path,
            record,
            file,
        };
        observation.verify()?;
        Ok(Some(observation))
    }
    pub fn record(&self) -> &SetupRecord {
        &self.record.record
    }
    pub fn verify(&self) -> Result<()> {
        self.namespace.verify()?;
        ensure!(
            self.namespace
                .read_json_with_identity::<BoundRecord>(&self.path)?
                == (self.record.clone(), self.file),
            CONFLICT
        );
        self.namespace.verify()
    }
}
