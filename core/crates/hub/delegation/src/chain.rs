use crate::{DelegationError, PeerIdentity, SignedGrant, constants::*};
use serde::{Deserialize, Serialize};
use yolo_hub_routing::Rights;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Vec<SignedGrant>", into = "Vec<SignedGrant>")]
pub struct DelegationChain(Vec<SignedGrant>);

impl DelegationChain {
    pub fn new(links: Vec<SignedGrant>) -> Result<Self, DelegationError> {
        links.try_into()
    }
    pub fn links(&self) -> &[SignedGrant] {
        &self.0
    }
    pub fn recipient(&self) -> PeerIdentity {
        self.0
            .last()
            .expect("validated nonempty delegation")
            .fields()
            .claims
            .recipient
    }
    pub fn origin(&self) -> PeerIdentity {
        self.0[0].fields().claims.origin
    }
    pub fn policy_epoch(&self) -> u64 {
        self.0[0].fields().claims.policy_epoch
    }

    /// Callers must obtain the pinned origin and current policy epoch independently
    /// of this chain. Possession of a correctly signed chain is not a trust anchor.
    pub fn authorize(
        &self,
        origin: PeerIdentity,
        epoch: u64,
        recipient: PeerIdentity,
        now: u64,
        required: &Rights,
    ) -> Result<(), DelegationError> {
        if self.origin() != origin {
            return Err(DelegationError::Origin);
        }
        if self.policy_epoch() != epoch {
            return Err(DelegationError::Revoked);
        }
        if self.recipient() != recipient {
            return Err(DelegationError::Recipient);
        }
        for link in &self.0 {
            let claims = &link.fields().claims;
            if now < claims.not_before_ms || now >= claims.expires_at_ms {
                return Err(DelegationError::Expired);
            }
        }
        if !self
            .0
            .last()
            .expect("validated nonempty delegation")
            .fields()
            .claims
            .rights
            .allows(required)
        {
            return Err(DelegationError::Escalation);
        }
        Ok(())
    }

    pub fn append(&self, grant: SignedGrant) -> Result<Self, DelegationError> {
        let mut chain = self.0.clone();
        chain.push(grant);
        Self::new(chain)
    }
}

impl TryFrom<Vec<SignedGrant>> for DelegationChain {
    type Error = DelegationError;
    fn try_from(links: Vec<SignedGrant>) -> Result<Self, Self::Error> {
        if links.is_empty() || links.len() > usize::from(MAX_DELEGATION_DEPTH) + FIRST_CHAIN_LINK {
            return Err(DelegationError::Path);
        }
        let root = &links[0].fields().claims;
        if root.issuer != root.origin || root.parent.is_some() {
            return Err(DelegationError::Origin);
        }
        let mut visited = vec![root.origin.hub()];
        for (index, link) in links.iter().enumerate() {
            link.verify()?;
            let claims = &link.fields().claims;
            if visited.contains(&claims.recipient.hub()) {
                return Err(DelegationError::Path);
            }
            visited.push(claims.recipient.hub());
            if index == 0 {
                continue;
            }
            let previous = &links[index - FIRST_CHAIN_LINK];
            let parent = &previous.fields().claims;
            if claims.issuer != parent.recipient
                || claims.origin != root.origin
                || claims.parent != Some(previous.fingerprint()?)
            {
                return Err(DelegationError::Path);
            }
            if claims.policy_epoch != root.policy_epoch
                || parent.remaining_reexports == NO_REEXPORTS
                || claims.remaining_reexports >= parent.remaining_reexports
                || !parent.rights.allows(&claims.rights)
                || claims.not_before_ms < parent.not_before_ms
                || claims.expires_at_ms > parent.expires_at_ms
            {
                return Err(DelegationError::Escalation);
            }
        }
        Ok(Self(links))
    }
}
impl From<DelegationChain> for Vec<SignedGrant> {
    fn from(value: DelegationChain) -> Self {
        value.0
    }
}
