use crate::{
    Federation,
    constants::*,
    discovery_channel::{self, DiscoveryRefresh},
    node::now_ms,
    registry::Link,
};
use yolo_hub_peer_diagnostics::EXPORT_DEPTH_EXHAUSTED;
use anyhow::{Context, Result, ensure};
use std::{collections::BTreeSet, sync::atomic::Ordering};
use tokio::time::Instant;
use uuid::Uuid;
use yolo_hub_delegation::{DelegationChain, GrantClaims, PeerIdentity, SignedGrant};
use yolo_hub_identity::HubId;
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
            .request(
                PeerRequest::Discover {
                    request_id,
                    version: PEER_PROTOCOL_VERSION,
                    allowed_origins,
                },
                deadline,
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
        book.install(link.peer, link.id, revision, origins, now_ms()?)?;
        let _ = self.observation_cache.record(book.history());
        self.require_open()?;
        Ok(DiscoveryRefresh::Updated)
    }

    pub(crate) async fn directory(
        &self,
        peer: PeerIdentity,
        request_id: Uuid,
        version: u16,
        allowed_origins: Vec<HubId>,
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
            .source()
            .filter(|source| source.payload.policy_epoch == self.policy_epoch)
        {
            if let Some(policy) = self
                .config
                .export(peer.hub(), self.hub_id())
                .filter(|policy| allowed.contains(&policy.origin))
            {
                let claims = GrantClaims {
                    grant_id: Uuid::new_v4(),
                    issuer: self.state.peer_identity(),
                    recipient: peer,
                    origin: self.state.peer_identity(),
                    policy_epoch: self.policy_epoch,
                    not_before_ms: now.saturating_sub(yolo_hub_peer_protocol::MAX_CLOCK_SKEW_MS),
                    expires_at_ms: now.checked_add(GRANT_LIFETIME_MS).context(EXHAUSTED)?,
                    remaining_reexports: policy.remaining_reexports,
                    rights: policy.rights.clone(),
                    parent: None,
                };
                let authority =
                    DelegationChain::new(vec![SignedGrant::issue(claims, self.state.as_ref())?])?;
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
