use anyhow::{Context, Result, ensure};
use std::sync::Mutex;
use uuid::Uuid;
use yolo_hub_delegation::{DelegationChain, GrantClaims, PeerIdentity, SignedGrant, Signer};
use yolo_hub_federation_settings::ExportPolicy;
use yolo_hub_peer_protocol::MAX_CLOCK_SKEW_MS;

/// The authenticated connection retains only the direct grant for its own peer.
#[derive(Default)]
pub struct LocalGrant(Mutex<Option<DelegationChain>>);

pub const GRANT_LIFETIME_MS: u64 = 300_000;
const RENEW_BEFORE_MS: u64 = GRANT_LIFETIME_MS / 2;

impl LocalGrant {
    pub fn for_peer(
        &self,
        signer: &impl Signer,
        peer: PeerIdentity,
        epoch: u64,
        policy: &ExportPolicy,
        now: u64,
    ) -> Result<DelegationChain> {
        let issuer = signer.identity();
        ensure!(
            policy.origin == issuer.hub(),
            "direct export origin differs from signer"
        );
        let expires = now
            .checked_add(GRANT_LIFETIME_MS)
            .context("direct grant expiry overflows the clock")?;
        let mut retained = self
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!("direct grant lock poisoned"))?;
        if let Some(chain) = retained.as_ref() {
            let claims = &chain.links()[0].fields().claims;
            if claims.issuer == issuer
                && claims.recipient == peer
                && claims.policy_epoch == epoch
                && claims.rights == policy.rights
                && claims.remaining_reexports == policy.remaining_reexports
                && claims.expires_at_ms <= expires
                && claims.expires_at_ms.saturating_sub(now) > RENEW_BEFORE_MS
            {
                return Ok(chain.clone());
            }
        }
        let grant = SignedGrant::issue(
            GrantClaims {
                grant_id: Uuid::new_v4(),
                issuer,
                recipient: peer,
                origin: issuer,
                policy_epoch: epoch,
                not_before_ms: now.saturating_sub(MAX_CLOCK_SKEW_MS),
                expires_at_ms: expires,
                remaining_reexports: policy.remaining_reexports,
                rights: policy.rights.clone(),
                parent: None,
            },
            signer,
        )?;
        let chain = DelegationChain::new(vec![grant])?;
        *retained = Some(chain.clone());
        Ok(chain)
    }
}
