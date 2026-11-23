use crate::{EnrollmentRecord, constants::*, record::StoredRecord};
use anyhow::{Result, ensure};
use std::path::PathBuf;
use yolo_hub_delegation::PeerIdentity;
use yolo_remote_provisioning_protocol::RemoteBootstrapRequest;
use yolo_paths::{
    directory::PrivateDirectory, identity::FileIdentity, namespace_lease::NamespaceLease,
    private_file::io_kind,
};

pub struct Observation {
    pub(crate) namespace: NamespaceLease,
    pub(crate) path: PathBuf,
    pub(crate) identity: FileIdentity,
    pub(crate) stored: StoredRecord,
}
impl Observation {
    pub fn record(&self) -> &EnrollmentRecord {
        &self.stored.record
    }
    pub fn verify(&self) -> Result<()> {
        self.namespace.verify()?;
        let (stored, identity): (StoredRecord, FileIdentity) =
            self.namespace.read_json_with_identity(&self.path)?;
        ensure!(identity == self.identity && stored == self.stored, CHANGED);
        self.namespace.verify()
    }
}

pub(crate) fn inspect(
    parent: &PrivateDirectory,
    root: &PrivateDirectory,
    peer: PeerIdentity,
    request: &RemoteBootstrapRequest,
) -> Result<Option<Observation>> {
    parent.verify_path()?;
    root.verify_path()?;
    let entry_path = root.path().join(request.fields().request_id.to_string());
    let entry = match root.open_descendant(&entry_path) {
        Ok(entry) => entry,
        Err(error) if io_kind(&error) == Some(std::io::ErrorKind::NotFound) => {
            parent.verify_path()?;
            root.verify_path()?;
            return Ok(None);
        }
        Err(error) => return Err(error),
    };
    let namespace = NamespaceLease::from_directories(vec![
        (PARENT_NAMESPACE, parent.try_clone()?),
        (ROOT_NAMESPACE, root.try_clone()?),
        (ENTRY_NAMESPACE, entry.try_clone()?),
    ])?;
    let path = entry.path().join(RECORD_FILE);
    ensure!(!namespace.file_is_missing(&path)?, INCOMPLETE);
    let (stored, identity): (StoredRecord, FileIdentity) =
        namespace.read_json_with_identity(&path)?;
    stored.validate(parent, root, &entry)?;
    ensure!(
        stored.record.prepared.request == *request && stored.record.prepared.peer == peer,
        CONFLICT
    );
    let observation = Observation {
        namespace,
        path,
        identity,
        stored,
    };
    observation.verify()?;
    Ok(Some(observation))
}
