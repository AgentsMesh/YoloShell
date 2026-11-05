use crate::constants::PUBLIC_KEY_FIELD;
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

fn peer_document(peer: &Peer) -> serde_json::Value {
    serde_json::json!({
        "hub": peer.identity.hub().to_string(),
        (PUBLIC_KEY_FIELD): key_text(peer.identity),
        "dial": peer.dial.as_ref().map(|dial| serde_json::json!({
            "profile": dial.profile,
            "socket": dial.socket,
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
