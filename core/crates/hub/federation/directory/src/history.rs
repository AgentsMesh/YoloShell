use crate::{OriginBook, constants::*};
use anyhow::{Result, ensure};
use std::{collections::BTreeMap, sync::Arc};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_identity::HubId;
use yolo_hub_peer_protocol::{Catalogue, Purpose, Signed};
use yolo_hub_routing::RoutingLimits;

/// Authentic observations only; never delegated grants, connections or executable routes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationHistory {
    entry: PeerIdentity,
    catalogues: BTreeMap<HubId, Arc<Signed<Catalogue>>>,
}

impl ObservationHistory {
    pub fn new(
        entry: PeerIdentity,
        catalogues: Vec<Signed<Catalogue>>,
        limits: RoutingLimits,
    ) -> Result<Self> {
        ensure!(catalogues.len() <= limits.max_known_agents.get(), CAPACITY);
        let mut retained = BTreeMap::new();
        for catalogue in catalogues {
            ensure!(
                retained
                    .insert(catalogue.signer.hub(), Arc::new(catalogue))
                    .is_none(),
                HISTORY_DUPLICATE_ORIGIN
            );
        }
        let history = Self {
            entry,
            catalogues: retained,
        };
        history.validate(limits)?;
        Ok(history)
    }
    pub fn entry(&self) -> PeerIdentity {
        self.entry
    }
    pub fn catalogues(&self) -> impl ExactSizeIterator<Item = &Signed<Catalogue>> {
        self.catalogues.values().map(|catalogue| catalogue.as_ref())
    }
    pub fn validate(&self, limits: RoutingLimits) -> Result<()> {
        ensure!(
            self.catalogues.len() <= limits.max_known_agents.get(),
            CAPACITY
        );
        for (origin, catalogue) in &self.catalogues {
            ensure!(
                *origin == catalogue.signer.hub()
                    && *origin != self.entry.hub()
                    && catalogue.signer.key() != self.entry.key(),
                HISTORY_LOCAL_ORIGIN
            );
            catalogue.verify(Purpose::Catalogue, catalogue.signer)?;
            catalogue.payload.validate(catalogue.signer)?;
        }
        Ok(())
    }
}

impl OriginBook {
    pub fn history(&self) -> ObservationHistory {
        ObservationHistory {
            entry: self.local,
            catalogues: self.latest.clone(),
        }
    }

    /// Merge before activation. Unpinned history remains offline; changed pinned keys are dropped.
    pub fn restore_history(&mut self, history: ObservationHistory) -> Result<()> {
        ensure!(history.entry == self.local, HISTORY_ENTRY_MISMATCH);
        ensure!(
            self.links.is_empty() && self.proofs.is_empty(),
            HISTORY_TARGET_BUSY
        );
        history.validate(self.limits)?;
        let mut catalogues = self.latest.clone();
        for (origin, catalogue) in history.catalogues {
            if self
                .pins
                .get(&origin)
                .is_some_and(|pin| *pin != catalogue.signer)
            {
                continue;
            }
            if let Some(previous) = catalogues.get(&origin) {
                ensure!(
                    previous.signer == catalogue.signer,
                    HISTORY_IDENTITY_CONFLICT
                );
                let old = &previous.payload;
                let new = &catalogue.payload;
                if old.revision == new.revision {
                    ensure!(old == new, CONFLICTING_ORIGIN);
                }
                if new.revision <= old.revision || new.policy_epoch < old.policy_epoch {
                    continue;
                }
            }
            catalogues.insert(origin, catalogue);
            ensure!(
                catalogues.len() <= self.limits.max_known_agents.get(),
                CAPACITY
            );
        }
        self.latest = catalogues;
        Ok(())
    }
}
