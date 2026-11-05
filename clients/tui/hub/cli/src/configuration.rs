use crate::constants::*;
use crate::AddPeer;
use anyhow::{Context, Result, ensure};
use std::collections::BTreeSet;
use uuid::Uuid;
use yolo_hub_delegation::{ED25519_PUBLIC_KEY_BYTES, PeerIdentity};
use yolo_hub_federation_settings::{ExportPolicy, Peer, PeerAddress};
use yolo_hub_identity::HubId;
use yolo_hub_management_protocol::{ConfigurationChange, ConfigurationSnapshot};
use yolo_hub_routing::Right;

pub fn add_change(
    snapshot: &ConfigurationSnapshot,
    args: &AddPeer,
) -> Result<ConfigurationChange> {
    let local = snapshot.state.identity;
    let identity = identity(args.peer, &args.key)?;
    ensure!(identity.hub() != local.hub(), PEER_IS_LOCAL);
    ensure!(
        snapshot
            .state
            .configuration
            .peers
            .iter()
            .all(|peer| peer.identity.hub() != identity.hub()),
        PEER_ALREADY_CONFIGURED
    );
    let dial = match (&args.ssh_profile, &args.remote_socket) {
        (Some(profile), Some(socket)) => {
            Some(PeerAddress::ssh(profile.clone(), socket.clone())?)
        }
        (None, None) => None,
        _ => anyhow::bail!("{}", INVALID_PEER_DIAL),
    };
    let mut origins = args.exports.iter().copied().collect::<BTreeSet<_>>();
    if args.export_local {
        origins.insert(local.hub());
    }
    let exports = origins
        .into_iter()
        .map(|origin| ExportPolicy {
            origin,
            rights: [Right::Observe].into_iter().collect(),
            remaining_reexports: 0,
        })
        .collect();
    let mut configuration = snapshot.state.configuration.clone();
    let consume = dial.is_some() && !args.no_import;
    configuration.peers.push(Peer {
        identity,
        dial,
        consume,
        exports,
    });
    configuration.validate(local)?;
    Ok(ConfigurationChange {
        request_id: Uuid::new_v4(),
        expected_revision: snapshot.state.revision,
        configuration,
    })
}

fn identity(hub: HubId, key: &str) -> Result<PeerIdentity> {
    let key = key.trim();
    ensure!(
        key.len() == ED25519_PUBLIC_KEY_BYTES * 2 && key.is_ascii(),
        INVALID_PUBLIC_KEY
    );
    let mut bytes = [u8::default(); ED25519_PUBLIC_KEY_BYTES];
    for (index, byte) in bytes.iter_mut().enumerate() {
        let offset = index * 2;
        *byte = u8::from_str_radix(&key[offset..offset + 2], 16).context(INVALID_PUBLIC_KEY)?;
    }
    PeerIdentity::new(hub, bytes).context(INVALID_PUBLIC_KEY)
}
