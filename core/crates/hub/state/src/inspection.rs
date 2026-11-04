use crate::{
    HubState,
    constants::*,
    record::{Binding, Record},
};
use anyhow::{Context, Result};
use ring::signature::{Ed25519KeyPair, KeyPair};
use std::path::Path;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_identity::HubId;
use yolo_paths::{directory::PrivateDirectory, namespace_lease::NamespaceLease};

impl HubState {
    /// Read only the bound identity for local startup. The private key stays inside
    /// this component; invalid or cloned state is never replaced during discovery.
    pub fn inspect_identity(path: &Path, host_home: &Path) -> Result<Option<HubId>> {
        Self::inspect_peer_identity(path, host_home).map(|identity| identity.map(PeerIdentity::hub))
    }

    /// Project the persistent public key without exporting its private record.
    pub fn inspect_peer_identity(path: &Path, host_home: &Path) -> Result<Option<PeerIdentity>> {
        let path = yolo_paths::platform::platform_path(path);
        let directory = match PrivateDirectory::open(&path) {
            Ok(directory) => directory,
            Err(error)
                if error
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound) =>
            {
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        let binding = Binding::capture(&directory)?;
        let namespace = NamespaceLease::from_directories(vec![(STATE_NAMESPACE, directory)])?;
        let file = path.join(HUB_IDENTITY_FILE);
        if namespace.file_is_missing(&file)? {
            return Ok(None);
        }
        let record: Record = namespace.read_json(&file)?;
        let host_home = yolo_paths::coordination::canonical_logical_path(host_home)?;
        record.verify(&binding, &host_home)?;
        let key =
            Ed25519KeyPair::from_pkcs8(&record.key).map_err(|_| anyhow::anyhow!(KEY_FAILURE))?;
        let public = key.public_key().as_ref().try_into().context(KEY_FAILURE)?;
        let identity = PeerIdentity::new(record.hub_id, public).context(KEY_FAILURE)?;
        namespace.verify().context(KEY_FAILURE)?;
        Ok(Some(identity))
    }
}
