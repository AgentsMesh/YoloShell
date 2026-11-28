use crate::CatalogueChanges;
use anyhow::{Result, ensure};
use std::{collections::BTreeMap, sync::Mutex, time::Duration};
use uuid::Uuid;
use yolo_hub_federation_settings::Configuration;
use yolo_hub_identity::HubId;
use yolo_hub_peer_protocol::{
    DIRECTORY_WAIT_MS, MAX_DIRECTORY_ORIGINS, PEER_PROTOCOL_VERSION, PeerRequest,
};

/// Recheck proofs and transport health even if this peer has no publication changes.
const MAX_WAIT: Duration = Duration::from_millis(DIRECTORY_WAIT_MS);
type Snapshot = BTreeMap<HubId, Uuid>;

#[derive(Default)]
pub struct PeerDirectoryWatch {
    previous: Mutex<Option<Snapshot>>,
}

impl PeerDirectoryWatch {
    /// The first observation is immediate. The next one watches only origins
    /// this authenticated connection requested and the provider can export.
    pub async fn before(
        &self,
        changes: &CatalogueChanges,
        config: &Configuration,
        local: HubId,
        peer: HubId,
        request: &PeerRequest,
    ) -> Result<()> {
        let PeerRequest::Discover {
            request_id,
            version,
            allowed_origins,
        } = request
        else {
            anyhow::bail!("invalid directory watch request");
        };
        ensure!(
            !request_id.is_nil()
                && *version == PEER_PROTOCOL_VERSION
                && allowed_origins.len() <= MAX_DIRECTORY_ORIGINS,
            "invalid directory watch request"
        );
        let mut origins: Vec<_> = allowed_origins
            .iter()
            .copied()
            .filter(|id| {
                *id != peer
                    && (config.export(peer, *id).is_some()
                        || config.shared_export(*id, local).is_some())
            })
            .collect();
        origins.sort_unstable();
        origins.dedup();
        let prior = self
            .previous
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone();
        let mut watch = changes.versions.subscribe();
        let version = |state: &crate::catalogue_watch::RevisionState, id: &HubId| {
            state
                .origins
                .get(id)
                .map_or(changes.epoch, |version| version.advertisement)
        };
        if let Some(prior) = &prior {
            let changed = watch.wait_for(|state| {
                state.as_ref().is_none_or(|map| {
                    prior.len() != origins.len()
                        || origins
                            .iter()
                            .any(|id| prior.get(id) != Some(&version(map, id)))
                })
            });
            let _ = tokio::time::timeout(MAX_WAIT, changed).await;
        }
        let state = watch.borrow();
        let map = state
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("directory watch retired"))?;
        let snapshot = origins.iter().map(|id| (*id, version(map, id))).collect();
        *self.previous.lock().unwrap_or_else(|p| p.into_inner()) = Some(snapshot);
        Ok(())
    }
}
