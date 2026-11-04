use crate::state::State;
use std::collections::BTreeMap;
use yolo_hub_connector_protocol::ConnectorError;
use yolo_hub_connector_store::{Seed, Store};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_identity::{ConnectorGeneration, ConnectorId};
use yolo_paths::directory::PrivateDirectory;

pub struct Registry {
    pub(crate) identity: PeerIdentity,
    pub(crate) store: Store<State>,
    pub(crate) live: BTreeMap<ConnectorId, ConnectorGeneration>,
    pub(crate) cache: BTreeMap<ConnectorId, (Seed, State)>,
}
impl Registry {
    pub fn open(
        directory: PrivateDirectory,
        identity: PeerIdentity,
    ) -> Result<Self, ConnectorError> {
        let store = Store::open(directory, identity).map_err(|_| ConnectorError::Unavailable)?;
        let entries = store.entries().map_err(|_| ConnectorError::Unavailable)?;
        validate_ownership(&entries)?;
        let cache = entries
            .into_iter()
            .map(|entry: (Seed, State)| (entry.0.connector_id, entry))
            .collect();
        let mut registry = Self {
            identity,
            store,
            live: BTreeMap::new(),
            cache,
        };
        let entries = registry.cache.values().cloned().collect::<Vec<_>>();
        for (seed, mut state) in entries {
            if crate::observations::stale(&mut state)? {
                registry.save(&seed, state)?;
            }
        }
        Ok(registry)
    }
    pub(crate) fn entry(&self, id: ConnectorId) -> Result<(Seed, State), ConnectorError> {
        self.store
            .get(id)
            .map_err(|_| ConnectorError::Unavailable)?
            .ok_or(ConnectorError::NotFound)
    }
    pub(crate) fn entries(&self) -> Result<Vec<(Seed, State)>, ConnectorError> {
        let entries = self
            .store
            .entries()
            .map_err(|_| ConnectorError::Unavailable)?;
        validate_ownership(&entries)?;
        Ok(entries)
    }
    pub(crate) fn save(&mut self, seed: &Seed, state: State) -> Result<(), ConnectorError> {
        if self.store.update(seed.connector_id, state.clone()).is_err() {
            self.live.clear();
            return Err(ConnectorError::OutcomeUnknown);
        }
        self.cache.insert(seed.connector_id, (seed.clone(), state));
        Ok(())
    }
}

fn validate_ownership(entries: &[(Seed, State)]) -> Result<(), ConnectorError> {
    let mut agents = std::collections::BTreeSet::new();
    let mut terminals = std::collections::BTreeSet::new();
    for (seed, state) in entries {
        if !agents.insert(seed.agent)
            || (state.status == yolo_hub_connector_protocol::EnrollmentStatus::Enabled
                && !terminals.insert(seed.scope.terminal))
        {
            return Err(ConnectorError::Conflict);
        }
    }
    Ok(())
}
