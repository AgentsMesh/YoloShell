mod constants;
mod refresh;
#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod tests;
use anyhow::{Context, Result, ensure};
use constants::*;
use ring::digest;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use yolo_hub_delegation::{FIRST_POLICY_EPOCH, PeerIdentity};
use yolo_hub_federation_settings::Configuration;
use yolo_hub_peer_protocol::{CATALOGUE_SCHEMA_VERSION, Catalogue, Purpose, Signed};
use yolo_paths::{
    coordination::CoordinationLock, directory::PrivateDirectory, namespace_lease::NamespaceLease,
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema_version: u16,
    identity: PeerIdentity,
    directory: yolo_paths::identity::DurableObjectIdentity,
    config_digest: [u8; digest::SHA256_OUTPUT_LEN],
    revision: u64,
    policy_epoch: u64,
    latest: Option<Signed<Catalogue>>,
}
pub struct Publication {
    record: Record,
    path: PathBuf,
    namespace: NamespaceLease,
    lock: CoordinationLock,
}
impl Publication {
    pub fn open(
        directory: PrivateDirectory,
        identity: PeerIdentity,
        configuration: &Configuration,
    ) -> Result<Self> {
        let object = directory.durable_identity()?;
        let root = directory.path().to_path_buf();
        let lock = CoordinationLock::try_acquire(&root.join(LOCK_FILE), LOCK_BYTES)?
            .context(ALREADY_RUNNING)?;
        let namespace = NamespaceLease::from_directories(vec![(NAMESPACE, directory)])?;
        let path = root.join(RECORD_FILE);
        let bytes = yolo_hub_transport::encode(configuration)?;
        let config_digest = digest::digest(&digest::SHA256, &bytes)
            .as_ref()
            .try_into()?;
        let mut record = if namespace.file_is_missing(&path)? {
            Record {
                schema_version: RECORD_SCHEMA,
                identity,
                directory: object,
                config_digest,
                revision: 0,
                policy_epoch: FIRST_POLICY_EPOCH,
                latest: None,
            }
        } else {
            let mut record: Record = namespace.read_json(&path)?;
            ensure!(
                record.schema_version == RECORD_SCHEMA
                    && record.identity == identity
                    && record.directory == object
                    && record.policy_epoch >= FIRST_POLICY_EPOCH,
                INVALID_RECORD
            );
            if record
                .latest
                .as_ref()
                .is_some_and(|latest| latest.payload.schema_version != CATALOGUE_SCHEMA_VERSION)
            {
                // A previous catalogue shape cannot be verified under the
                // current canonical encoding. Keep the revision and let the
                // next refresh publish a new signed catalogue.
                record.latest = None;
            }
            if let Some(latest) = &record.latest {
                latest.verify(Purpose::Catalogue, identity)?;
                latest.payload.validate(identity)?;
                ensure!(
                    latest.payload.revision == record.revision
                        && latest.payload.policy_epoch <= record.policy_epoch,
                    INVALID_RECORD
                );
            }
            record
        };
        if record.config_digest != config_digest {
            record.config_digest = config_digest;
            record.policy_epoch = record
                .policy_epoch
                .checked_add(FIRST_POLICY_EPOCH)
                .context(EXHAUSTED)?;
        }
        namespace.write_json(&path, &record)?;
        Ok(Self {
            record,
            path,
            namespace,
            lock,
        })
    }
    pub fn policy_epoch(&self) -> u64 {
        self.record.policy_epoch
    }
    pub fn latest(&self) -> Option<&Signed<Catalogue>> {
        self.record.latest.as_ref()
    }
    pub fn verify(&self) -> Result<()> {
        self.lock.verify()?;
        self.namespace.verify()
    }
}
