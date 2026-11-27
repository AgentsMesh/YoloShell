use crate::constants::*;
use crate::creation::CreationEvidence;
use crate::persistent_namespace::PersistentNamespace;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_management_protocol::ConfigurationState;
use yolo_paths::{
    directory::PrivateDirectory, identity::DurableObjectIdentity, namespace_lease::NamespaceLease,
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub schema: u16,
    pub root: PathBuf,
    pub owner: u32,
    pub device: u64,
    pub peer: PeerIdentity,
    pub configuration: ConfigurationState,
    pub creation: CreationEvidence,
    pub proofs: BTreeMap<PathBuf, DurableObjectIdentity>,
    pub records: BTreeMap<PathBuf, Vec<u8>>,
    pub socket_namespaces: Vec<PersistentNamespace>,
}

impl Snapshot {
    pub fn load(root: &Path) -> Result<Self> {
        let directory = PrivateDirectory::open(root)?;
        let namespace = NamespaceLease::from_directories(vec![(NAMESPACE, directory)])?;
        let snapshot: Self = namespace.read_json(&root.join(BASELINE_FILE))?;
        ensure!(
            snapshot.schema == BASELINE_SCHEMA
                && snapshot.root == root
                && snapshot.owner == unsafe { libc::geteuid() },
            BINDING_CHANGED
        );
        Ok(snapshot)
    }

    pub fn verify(&self) -> Result<()> {
        self.creation.verify(&self.root, self.peer.hub())?;
        for namespace in &self.socket_namespaces {
            namespace.verify()?;
        }
        for (relative, expected) in &self.proofs {
            let path = if relative == Path::new(ROOT_PROOF) {
                self.root.clone()
            } else {
                self.root.join(relative)
            };
            let directory = PrivateDirectory::open(&path)?;
            ensure!(directory.durable_identity()? == *expected, PROOF_CHANGED);
        }
        for (relative, bytes) in &self.records {
            ensure!(
                std::fs::read(self.root.join(relative))? == *bytes,
                RECORD_CHANGED
            );
        }
        Ok(())
    }
}

pub fn device(root: &Path) -> Result<u64> {
    Ok(std::fs::metadata(root)?.dev())
}
