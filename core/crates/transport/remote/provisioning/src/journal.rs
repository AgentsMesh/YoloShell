use crate::{constants::*, record::Record};
use anyhow::{Result, ensure};
use std::path::PathBuf;
use yolo_paths::{
    directory::PrivateDirectory, identity::FileIdentity, namespace_lease::NamespaceLease,
    private_file::io_kind,
};
use yolo_remote_provisioning_protocol::ProvisionRequest;

pub(crate) struct Observation {
    pub namespace: NamespaceLease,
    pub path: PathBuf,
    pub file: FileIdentity,
    pub record: Record,
}
impl Observation {
    pub fn verify(&self) -> Result<()> {
        self.namespace.verify()?;
        let (record, file): (Record, FileIdentity) =
            self.namespace.read_json_with_identity(&self.path)?;
        ensure!(file == self.file && record == self.record, CHANGED);
        self.namespace.verify()
    }
}
pub(crate) fn inspect(
    parent: &PrivateDirectory,
    root: &PrivateDirectory,
    request: &ProvisionRequest,
) -> Result<Option<Observation>> {
    parent.verify_path()?;
    root.verify_path()?;
    let path = root.path().join(request.fields().request_id.to_string());
    let entry = match root.open_descendant(&path) {
        Ok(entry) => entry,
        Err(error) if io_kind(&error) == Some(std::io::ErrorKind::NotFound) => {
            parent.verify_path()?;
            root.verify_path()?;
            return Ok(None);
        }
        Err(error) => return Err(error),
    };
    let namespace = namespace(parent, root, &entry)?;
    let path = path.join(RECORD_FILE);
    let (record, file): (Record, FileIdentity) = namespace.read_json_with_identity(&path)?;
    record.validate()?;
    ensure!(
        record.parent == parent.object_identity()?
            && record.root == root.object_identity()?
            && record.entry == entry.object_identity()?,
        CHANGED
    );
    ensure!(record.request == *request, CONFLICT);
    let result = Observation {
        namespace,
        path,
        file,
        record,
    };
    result.verify()?;
    Ok(Some(result))
}
pub(crate) fn namespace(
    parent: &PrivateDirectory,
    root: &PrivateDirectory,
    entry: &PrivateDirectory,
) -> Result<NamespaceLease> {
    NamespaceLease::from_directories(vec![
        (PARENT_NAMESPACE, parent.try_clone()?),
        (ROOT_NAMESPACE, root.try_clone()?),
        (ENTRY_NAMESPACE, entry.try_clone()?),
    ])
}
