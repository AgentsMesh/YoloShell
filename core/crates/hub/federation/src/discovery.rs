use crate::{
    Federation,
    constants::*,
    discovery_channel::{self, DiscoveryRefresh},
    node::now_ms,
    registry::Link,
};
use anyhow::{Context, Result, ensure};
use std::{collections::BTreeSet, sync::atomic::Ordering};
use tokio::time::Instant;
use uuid::Uuid;
use yolo_hub_delegation::{GrantClaims, PeerIdentity, SignedGrant};
use yolo_hub_identity::HubId;
use yolo_hub_peer_diagnostics::EXPORT_DEPTH_EXHAUSTED;
use yolo_hub_peer_protocol::{
    ExportedOrigin, FIRST_REVISION, MAX_DIRECTORY_ORIGINS, MAX_DIRECTORY_ROUTES,
    PEER_PROTOCOL_VERSION, PeerReply, PeerRequest, route,
};

impl Federation {
    pub(crate) async fn discover(&self, link: &Link) -> Result<DiscoveryRefresh> {
        self.require_open()?;
        let deadline = Instant::now() + OPERATION_TIMEOUT;
        let request_id = Uuid::new_v4();
        let allowed_origins = self.book.lock().await.allowed_origins();
        self.require_open()?;
        let response = link
            .client
            .observe_directory(
                PeerRequest::Discover {
                    request_id,
                    version: PEER_PROTOCOL_VERSION,
                    allowed_origins,
                },
                deadline,
                now_ms()?.saturating_add(yolo_hub_peer_protocol::DIRECTORY_WAIT_MS + POLL_INTERVAL.as_millis() as u64),
            )
            .await;
        let Some(response) = discovery_channel::classify(response)? else {
            return Ok(DiscoveryRefresh::DeferredLocalAdmission);
        };
        ensure!(response.updates.is_none(), INVALID_DISCOVERY);
        let PeerReply::Directory {
            request_id: observed,
            revision,
            origins,
        } = response.reply
        else {
            anyhow::bail!(INVALID_DISCOVERY);
        };
        ensure!(request_id == observed, INVALID_DISCOVERY);
        let mut book = self.book.lock().await;
        self.require_open()?;
        if book.install_observations(link.peer, link.id, revision, origins, now_ms()?)? {
            let _ = self.observation_cache.record(book.history());
        }
        self.require_open()?;
        Ok(DiscoveryRefresh::Updated)
    }

    pub(crate) async fn directory(
        &self,
        peer: PeerIdentity,
        request_id: Uuid,
        version: u16,
        allowed_origins: Vec<HubId>,
        local_grant: &yolo_hub_federation_directory::LocalGrant,
    ) -> Result<PeerReply> {
        self.require_open()?;
        ensure!(
            version == PEER_PROTOCOL_VERSION
                && !request_id.is_nil()
                && allowed_origins.len() <= MAX_DIRECTORY_ORIGINS,
            INVALID_DISCOVERY
        );
        let allowed: BTreeSet<_> = allowed_origins.into_iter().collect();
        let now = now_ms()?;
        let mut origins = Vec::new();
        if let Some(source) = self
            .latest
            .borrow()
            .clone()
            .filter(|source| source.payload.policy_epoch == self.policy_epoch)
        {
            let policy = self
                .config
                .export(peer.hub(), self.hub_id())
                .cloned()
                .or_else(|| self.config.shared_export(self.hub_id(), self.hub_id()));
            if let Some(policy) = policy.filter(|policy| allowed.contains(&policy.origin)) {
                let authority = local_grant.for_peer(
                    self.state.as_ref(),
                    peer,
                    self.policy_epoch,
                    &policy,
                    now,
                )?;
                origins.push(ExportedOrigin {
                    catalogue: (*source).clone(),
                    authority,
                    reachable: true,
                });
            }
        }
        let book = self.book.lock().await;
        self.require_open()?;
        for candidate in book.exportable(now)? {
            let owner = candidate.catalogue.signer.hub();
            let Some(policy) = self
                .config
                .export(peer.hub(), owner)
                .filter(|_| allowed.contains(&owner))
            else {
                continue;
            };
            if route(&candidate.authority).contains(&peer.hub()) {
                continue;
            }
            let parent = candidate
                .authority
                .links()
                .last()
                .context(INVALID_DISCOVERY)?;
            let previous = &parent.fields().claims;
            let Some(remaining) = previous.remaining_reexports.checked_sub(1) else {
                if self
                    .export_denials
                    .denied(peer.hub(), owner, self.policy_epoch)
                {
                    tracing::warn!(
                        message = EXPORT_DEPTH_EXHAUSTED,
                        peer = %peer.hub(),
                        origin = %owner,
                        policy_epoch = self.policy_epoch,
                        remaining_reexports = previous.remaining_reexports,
                    );
                }
                continue;
            };
            let claims = GrantClaims {
                grant_id: Uuid::new_v4(),
                issuer: self.state.peer_identity(),
                recipient: peer,
                origin: previous.origin,
                policy_epoch: previous.policy_epoch,
                not_before_ms: previous.not_before_ms,
                expires_at_ms: previous.expires_at_ms,
                remaining_reexports: remaining.min(policy.remaining_reexports),
                rights: previous.rights.intersection(&policy.rights),
                parent: Some(parent.fingerprint()?),
            };
            let child = SignedGrant::issue(claims, self.state.as_ref())?;
            let authority = candidate.authority.append(child)?;
            let current = book.latest_for(owner).context(INVALID_DISCOVERY)?;
            origins.push(ExportedOrigin {
                catalogue: (*current).clone(),
                authority,
                reachable: true,
            });
            self.export_denials
                .exported(peer.hub(), owner, self.policy_epoch);
        }
        ensure!(origins.len() <= MAX_DIRECTORY_ROUTES, INVALID_DISCOVERY);
        self.require_open()?;
        let revision = self
            .directory_revision
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                current.checked_add(FIRST_REVISION)
            })
            .map_err(|_| anyhow::anyhow!(EXHAUSTED))?
            + FIRST_REVISION;
        Ok(PeerReply::Directory {
            request_id,
            revision,
            origins,
        })
    }
}
