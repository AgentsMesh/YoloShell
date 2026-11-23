use crate::OriginBook;
use anyhow::Result;
use std::{collections::BTreeMap, sync::Arc};
use yolo_hub_identity::HubId;
use yolo_hub_peer_protocol::{Catalogue, ExportedOrigin, FIRST_REVISION, Signed};
use yolo_hub_routing::{Grant, HubCatalog, Right, Rights, RouteAdvertisement};

impl OriginBook {
    pub fn route_for(
        &self,
        origin: HubId,
        required: &Rights,
        now: u64,
    ) -> Result<Option<Arc<ExportedOrigin>>> {
        let routes = self.build_routes(&self.proofs, &self.latest, now)?;
        Ok(routes
            .route_for(origin, required)
            .and_then(|route| self.proofs.get(&route.path))
            .cloned())
    }
    pub fn exportable(&self, now: u64) -> Result<Vec<Arc<ExportedOrigin>>> {
        let routes = self.build_routes(&self.proofs, &self.latest, now)?;
        Ok(self
            .proofs
            .iter()
            .filter(|(path, origin)| {
                routes
                    .route_for(
                        origin.catalogue.signer.hub(),
                        &origin
                            .authority
                            .links()
                            .last()
                            .unwrap()
                            .fields()
                            .claims
                            .rights,
                    )
                    .is_some_and(|selected| &selected.path == *path)
            })
            .map(|(_, origin)| origin.clone())
            .collect())
    }

    pub(crate) fn build_routes(
        &self,
        proofs: &BTreeMap<Vec<HubId>, Arc<ExportedOrigin>>,
        latest: &BTreeMap<HubId, Arc<Signed<Catalogue>>>,
        now: u64,
    ) -> Result<HubCatalog> {
        let mut catalog = HubCatalog::new(self.local.hub(), self.limits);
        let mut advertisements: BTreeMap<HubId, Vec<RouteAdvertisement<HubId>>> = BTreeMap::new();
        let mut ceilings: BTreeMap<(HubId, HubId), Grant> = BTreeMap::new();
        for (path, origin) in proofs {
            let owner = origin.catalogue.signer.hub();
            let Some(current) = latest.get(&owner) else {
                continue;
            };
            if !origin.reachable
                || origin
                    .authority
                    .authorize(
                        current.signer,
                        current.payload.policy_epoch,
                        self.local,
                        now,
                        &[Right::Observe].into_iter().collect(),
                    )
                    .is_err()
            {
                continue;
            }
            let peer = path[1];
            let claims = &origin.authority.links().last().unwrap().fields().claims;
            let grant = Grant {
                rights: claims.rights.clone(),
                remaining_reexports: usize::from(claims.remaining_reexports),
            };
            let ceiling = ceilings
                .entry((peer, owner))
                .or_insert_with(|| grant.clone());
            ceiling.rights = ceiling.rights.union(&grant.rights);
            ceiling.remaining_reexports =
                ceiling.remaining_reexports.max(grant.remaining_reexports);
            advertisements
                .entry(peer)
                .or_default()
                .push(RouteAdvertisement {
                    address: owner,
                    path: path[1..].to_vec(),
                    grant,
                });
        }
        for ((peer, owner), ceiling) in ceilings {
            catalog.authorize(peer, owner, ceiling)?;
        }
        for (peer, advertisements) in advertisements {
            let connection = catalog.connect(peer)?;
            catalog.replace_from(connection, FIRST_REVISION, advertisements)?;
        }
        Ok(catalog)
    }
}
