use crate::constants::{EXPIRES_AT_MS_FIELD, PUBLIC_KEY_FIELD};
use anyhow::Result;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_federation_settings::Peer;
use yolo_hub_identity::HubId;
use yolo_hub_management_protocol::ConfigurationSnapshot;

pub(crate) fn add_json(snapshot: &ConfigurationSnapshot, hub: HubId) -> Result<String> {
    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "revision": snapshot.state.revision,
        "applied": snapshot.applied,
        "peer": snapshot.state.configuration.peer(hub).map(peer_document),
    }))?)
}

pub(crate) fn list_json(snapshot: &ConfigurationSnapshot) -> Result<String> {
    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "revision": snapshot.state.revision,
        "applied": snapshot.applied,
        "peers": snapshot.state.configuration.peers.iter().map(peer_document).collect::<Vec<_>>(),
    }))?)
}

pub(crate) fn listen_json(snapshot: &ConfigurationSnapshot) -> Result<String> {
    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "revision": snapshot.state.revision,
        "applied": snapshot.applied,
        "configured": snapshot.state.configuration.listen,
        "listen_address": snapshot.listen_address,
    }))?)
}

pub(crate) fn invite_json(
    snapshot: &ConfigurationSnapshot,
    invite: &yolo_hub_enrollment_protocol::MintedInvite,
) -> Result<String> {
    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "revision": snapshot.state.revision,
        "hub": snapshot.state.identity.hub().to_string(),
        "invite_id": invite.invite_id,
        "code": invite.code,
        (EXPIRES_AT_MS_FIELD): invite.expires_at_ms,
    }))?)
}

pub(crate) fn join_json(
    snapshot: &ConfigurationSnapshot,
    dial: &yolo_hub_enrollment_protocol::InviteDial,
    reply: &yolo_hub_enrollment_wire::JoinReply,
    changed: bool,
    expires_at_ms: u64,
) -> Result<String> {
    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "changed": changed,
        "local_revision": snapshot.state.revision,
        "hub": reply.hub.hub().to_string(),
        "peer_revision": reply.revision,
        "peer_applied": reply.applied,
        "dial": {
            "host": dial.host,
            "port": dial.port.get(),
        },
        (EXPIRES_AT_MS_FIELD): expires_at_ms,
    }))?)
}

fn peer_document(peer: &Peer) -> serde_json::Value {
    serde_json::json!({
        "hub": peer.identity.hub().to_string(),
        (PUBLIC_KEY_FIELD): key_text(peer.identity),
        "dial": peer.dial.as_ref().map(|dial| serde_json::json!({
            "host": dial.host,
            "port": dial.port.get(),
        })),
        "consume": peer.consume,
        "exports": peer.exports.iter().map(|policy| serde_json::json!({
            "origin": policy.origin.to_string(),
            "rights": policy.rights,
            "remaining_reexports": policy.remaining_reexports,
        })).collect::<Vec<_>>(),
    })
}

fn key_text(identity: PeerIdentity) -> String {
    identity
        .key()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
