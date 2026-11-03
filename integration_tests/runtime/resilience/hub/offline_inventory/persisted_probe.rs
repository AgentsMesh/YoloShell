use super::{
    constants::*,
    persisted_types::{Index, Record},
};
use anyhow::{Context, Result, ensure};
use serde::de::DeserializeOwned;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use yolo_hub_agent_protocol::AgentSnapshot;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_identity::HubId;
use yolo_hub_observation_store::OBSERVATION_INDEX_FILE;
use yolo_runtime::paths::{
    constants::MAX_DURABLE_JSON_BYTES,
    identity::{DurableObjectIdentity, FileIdentity},
    namespace_lease::NamespaceLease,
};

pub struct Probe {
    pub namespace: NamespaceLease,
    pub directory: PathBuf,
    pub identity: DurableObjectIdentity,
    pub owner: PeerIdentity,
    pub origins: BTreeMap<HubId, PeerIdentity>,
    pub expected: Vec<AgentSnapshot>,
}
impl Probe {
    pub fn published(&self) -> Result<bool> {
        self.namespace.verify()?;
        let path = self.directory.join(OBSERVATION_INDEX_FILE);
        if self.namespace.file_is_missing(&path)? {
            return Ok(false);
        }
        let Some((index, index_version)) = self.read::<Index>(&path)? else {
            return Ok(false);
        };
        index.validate(self.owner, self.identity)?;
        let mut observed = Vec::new();
        let mut versions = Vec::new();
        for origin in &index.origins {
            let peer = *self
                .origins
                .get(origin)
                .context("unexpected indexed origin in cache fixture")?;
            let path = self
                .directory
                .join(origin.to_string())
                .with_extension(CACHE_RECORD_EXTENSION);
            let Some((record, version)) = self.read::<Record>(&path)? else {
                return Ok(false);
            };
            record.validate(self.owner, self.identity, peer)?;
            observed.extend(record.catalogue.payload.agents.iter().cloned());
            versions.push((path, version, peer, record.catalogue));
        }
        for (path, previous, peer, previous_catalogue) in versions {
            let Some((record, current)) = self.read::<Record>(&path)? else {
                return Ok(false);
            };
            record.validate(self.owner, self.identity, peer)?;
            if current != previous || record.catalogue != previous_catalogue {
                return Ok(false);
            }
        }
        let Some((current, version)) = self.read::<Index>(&path)? else {
            return Ok(false);
        };
        current.validate(self.owner, self.identity)?;
        if version != index_version || current != index {
            return Ok(false);
        }
        self.namespace.verify()?;
        if self
            .origins
            .keys()
            .any(|origin| !index.origins.contains(origin))
        {
            return Ok(false);
        }
        Ok(self.expected.iter().all(|expected| {
            observed.iter().any(|actual| {
                actual.cursor().target() == expected.cursor().target()
                    && actual.fields().terminal == expected.fields().terminal
                    && actual.fields().working_directory == expected.fields().working_directory
            })
        }))
    }

    fn read<T: DeserializeOwned>(&self, path: &Path) -> Result<Option<(T, FileIdentity)>> {
        let entry = self.namespace.file(path)?;
        let previous = entry
            .inspect(MAX_DURABLE_JSON_BYTES)?
            .context("indexed cache file is missing")?;
        match self.namespace.read_json_with_identity(path) {
            Ok(value) => Ok(Some(value)),
            Err(error) => {
                // A concurrent atomic replacement is pending, never a successful read.
                // Invalid JSON, lost directories, permissions and stable corruption fail.
                if error.downcast_ref::<serde_json::Error>().is_some() {
                    return Err(error);
                }
                self.namespace.verify()?;
                let current = entry
                    .inspect(MAX_DURABLE_JSON_BYTES)?
                    .context("indexed cache file disappeared")?;
                ensure!(current != previous, error);
                Ok(None)
            }
        }
    }
}
