use crate::{PeerError, Purpose, Signed, constants::*};
use serde::{Deserialize, Serialize};
use yolo_hub_delegation::{DelegationChain, PeerIdentity};
use yolo_hub_identity::HubId;
use yolo_hub_routing::Rights;
use yolo_hub_rpc_protocol::{Command, Request};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Call {
    pub request: Request,
    pub authority: DelegationChain,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
}

impl Signed<Call> {
    pub fn admit(
        &self,
        local: HubId,
        peer: PeerIdentity,
        origin: PeerIdentity,
        policy_epoch: u64,
        now: u64,
    ) -> Result<(), PeerError> {
        self.verify(Purpose::Call, self.payload.authority.recipient())?;
        let call = &self.payload;
        if call.request.fields().command.peer_forbidden() {
            return Err(PeerError::Route);
        }
        if call.request.fields().destination != origin.hub() {
            return Err(PeerError::Route);
        }
        if call.expires_at_ms <= call.issued_at_ms
            || call.expires_at_ms - call.issued_at_ms > MAX_CALL_LIFETIME_MS
        {
            return Err(PeerError::Bounds);
        }
        if now.saturating_add(MAX_CLOCK_SKEW_MS) < call.issued_at_ms || now >= call.expires_at_ms {
            return Err(PeerError::Expired);
        }
        let required = required_rights(&call.request.fields().command).ok_or(PeerError::Route)?;
        call.authority
            .authorize(origin, policy_epoch, self.signer, now, &required)?;
        let mut identities = call
            .authority
            .links()
            .iter()
            .rev()
            .map(|grant| grant.fields().claims.recipient)
            .collect::<Vec<_>>();
        identities.push(origin);
        let position = identities
            .iter()
            .position(|identity| identity.hub() == local)
            .ok_or(PeerError::Route)?;
        if position == 0 || identities[position - 1] != peer {
            return Err(PeerError::Route);
        }
        Ok(())
    }
    pub fn next_hop(&self, local: HubId) -> Option<HubId> {
        let path = route(&self.payload.authority);
        let position = path.iter().position(|hub| *hub == local)?;
        path.get(position + 1).copied()
    }
}

pub fn route(authority: &DelegationChain) -> Vec<HubId> {
    authority
        .links()
        .iter()
        .rev()
        .map(|grant| grant.fields().claims.recipient.hub())
        .chain(std::iter::once(authority.origin().hub()))
        .collect()
}

/// Rights a remote command must present. `None` means the command is local
/// management and therefore can never be delegated over a peer connection.
pub fn required_rights(command: &Command) -> Option<Rights> {
    command.method().required_rights()
}
