use crate::constants::*;
use crate::PinMachine;
use anyhow::{Context, Result, ensure};
use std::collections::BTreeSet;
use uuid::Uuid;
use yolo_hub_delegation::{ED25519_PUBLIC_KEY_BYTES, PeerIdentity};
use yolo_hub_federation_settings::{ExportPolicy, Peer, PeerAddress};
use yolo_hub_identity::HubId;
use yolo_hub_management_protocol::{ConfigurationChange, ConfigurationSnapshot};
use yolo_hub_routing::Right;

pub fn pin_change(
    snapshot: &ConfigurationSnapshot,
    args: &PinMachine,
) -> Result<ConfigurationChange> {
    let local = snapshot.state.identity;
    let identity = identity(args.machine, &args.key)?;
    ensure!(identity.hub() != local.hub(), MACHINE_IS_LOCAL);
    ensure!(
        snapshot
            .state
            .configuration
            .peers
            .iter()
            .all(|peer| peer.identity.hub() != identity.hub()),
        MACHINE_ALREADY_CONFIGURED
    );
    let dial = match (&args.ssh, &args.remote_socket) {
        (Some(target), Some(socket)) => {
            Some(PeerAddress::ssh(target.clone(), socket.clone())?)
        }
        (None, None) => None,
        _ => anyhow::bail!("{}", INVALID_MACHINE_DIAL),
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
    let consume = dial.is_some() && !args.pin_only;
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

pub fn import_change(
    snapshot: &ConfigurationSnapshot,
    target: String,
    socket: std::path::PathBuf,
    identity: PeerIdentity,
    pin_only: bool,
) -> Result<Option<ConfigurationChange>> {
    let local = snapshot.state.identity;
    ensure!(identity.hub() != local.hub(), MACHINE_IS_LOCAL);
    let dial = PeerAddress::ssh(target, socket)?;
    let mut configuration = snapshot.state.configuration.clone();
    match configuration
        .peers
        .iter_mut()
        .find(|peer| peer.identity.hub() == identity.hub())
    {
        Some(peer) => {
            ensure!(peer.identity == identity, MACHINE_IDENTITY_CHANGED);
            peer.dial = Some(dial);
            peer.consume = !pin_only;
        }
        None => configuration.peers.push(Peer {
            identity,
            dial: Some(dial),
            consume: !pin_only,
            exports: Vec::new(),
        }),
    }
    configuration.validate(local)?;
    if configuration == snapshot.state.configuration {
        return Ok(None);
    }
    Ok(Some(ConfigurationChange {
        request_id: Uuid::new_v4(),
        expected_revision: snapshot.state.revision,
        configuration,
    }))
}

pub fn remove_change(
    snapshot: &ConfigurationSnapshot,
    hub: HubId,
) -> Result<ConfigurationChange> {
    let mut configuration = snapshot.state.configuration.clone();
    let before = configuration.peers.len();
    configuration.peers.retain(|peer| peer.identity.hub() != hub);
    ensure!(configuration.peers.len() != before, MACHINE_NOT_CONFIGURED);
    configuration.validate(snapshot.state.identity)?;
    Ok(ConfigurationChange {
        request_id: Uuid::new_v4(),
        expected_revision: snapshot.state.revision,
        configuration,
    })
}

pub(crate) fn identity(hub: HubId, key: &str) -> Result<PeerIdentity> {
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
