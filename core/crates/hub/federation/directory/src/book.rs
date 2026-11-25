use crate::constants::*;
use anyhow::{Context, Result, ensure};
use std::{collections::BTreeMap, sync::Arc};
use uuid::Uuid;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_identity::HubId;
use yolo_hub_peer_protocol::{Catalogue, ExportedOrigin, MAX_DIRECTORY_ROUTES, Signed, route};
use yolo_hub_routing::RoutingLimits;

pub(crate) struct Link {
    pub id: Uuid,
    pub revision: u64,
}
pub struct OriginBook {
    pub(crate) local: PeerIdentity,
    pub(crate) pins: BTreeMap<HubId, PeerIdentity>,
    pub(crate) limits: RoutingLimits,
    pub(crate) links: BTreeMap<HubId, Link>,
    pub(crate) proofs: BTreeMap<Vec<HubId>, Arc<ExportedOrigin>>,
    pub(crate) latest: BTreeMap<HubId, Arc<Signed<Catalogue>>>,
}
impl OriginBook {
    pub fn new(
        local: PeerIdentity,
        origins: Vec<PeerIdentity>,
        limits: RoutingLimits,
    ) -> Result<Self> {
        let mut pins = BTreeMap::new();
        for pin in origins {
            ensure!(pin.hub() != local.hub(), INVALID_PINS);
            ensure!(pins.insert(pin.hub(), pin).is_none(), INVALID_PINS);
        }
        Ok(Self {
            local,
            pins,
            limits,
            links: BTreeMap::new(),
            proofs: BTreeMap::new(),
            latest: BTreeMap::new(),
        })
    }
    pub fn pin(&self, origin: HubId) -> Option<PeerIdentity> {
        self.pins.get(&origin).copied()
    }
    pub fn allowed_origins(&self) -> Vec<HubId> {
        self.pins.keys().copied().collect()
    }
    pub fn latest_for(&self, origin: HubId) -> Option<Arc<Signed<Catalogue>>> {
        self.latest.get(&origin).cloned()
    }

    pub fn connect(&mut self, peer: PeerIdentity, link: Uuid) -> Result<()> {
        ensure!(self.pins.get(&peer.hub()) == Some(&peer), UNKNOWN_PEER);
        ensure!(!link.is_nil(), STALE_LINK);
        self.proofs
            .retain(|path, _| path.get(1) != Some(&peer.hub()));
        self.links.insert(
            peer.hub(),
            Link {
                id: link,
                revision: 0,
            },
        );
        Ok(())
    }
    pub fn disconnect(&mut self, peer: HubId, link: Uuid) -> Result<()> {
        self.require_link(peer, link)?;
        self.proofs.retain(|path, _| path.get(1) != Some(&peer));
        self.links.remove(&peer);
        Ok(())
    }

    pub fn install(
        &mut self,
        peer: PeerIdentity,
        link: Uuid,
        revision: u64,
        origins: Vec<ExportedOrigin>,
        now: u64,
    ) -> Result<()> {
        ensure!(self.pins.get(&peer.hub()) == Some(&peer), UNKNOWN_PEER);
        self.require_link(peer.hub(), link)?;
        ensure!(revision > self.links[&peer.hub()].revision, STALE_REVISION);
        ensure!(origins.len() <= MAX_DIRECTORY_ROUTES, CAPACITY);
        let mut proofs = self.proofs.clone();
        let mut latest = self.latest.clone();
        proofs.retain(|path, _| path.get(1) != Some(&peer.hub()));
        let mut inserted = std::collections::BTreeSet::new();
        for origin in origins {
            let owner = origin.catalogue.signer;
            let pin = self.pin(owner.hub()).context(UNKNOWN_ORIGIN)?;
            origin.verify(self.local, peer, pin, now)?;
            let path = route(&origin.authority);
            ensure!(inserted.insert(path.clone()), DUPLICATE_PATH);
            let origin = Arc::new(origin);
            if let Some(previous) = latest.get(&owner.hub()) {
                let old = &previous.payload;
                let new = &origin.catalogue.payload;
                if old.revision == new.revision {
                    ensure!(
                        previous.payload == origin.catalogue.payload,
                        CONFLICTING_ORIGIN
                    );
                }
                if old.revision < new.revision && old.policy_epoch <= new.policy_epoch {
                    latest.insert(owner.hub(), Arc::new(origin.catalogue.clone()));
                }
            } else {
                latest.insert(owner.hub(), Arc::new(origin.catalogue.clone()));
            }
            proofs.insert(path, origin);
        }
        ensure!(latest.len() <= self.limits.max_known_agents.get(), CAPACITY);
        self.build_routes(&proofs, &latest, now)?;
        self.proofs = proofs;
        self.latest = latest;
        self.links
            .get_mut(&peer.hub())
            .context(STALE_LINK)?
            .revision = revision;
        Ok(())
    }
    fn require_link(&self, peer: HubId, id: Uuid) -> Result<()> {
        ensure!(
            self.links.get(&peer).is_some_and(|link| link.id == id),
            STALE_LINK
        );
        Ok(())
    }
}
