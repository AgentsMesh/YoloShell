use crate::constants::*;
use crate::{AddPeer, InvitePeer};
use anyhow::{Context, Result, ensure};
use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::num::NonZeroU16;
use uuid::Uuid;
use yolo_hub_delegation::{ED25519_PUBLIC_KEY_BYTES, PeerIdentity};
use yolo_hub_enrollment_protocol::{InviteClaims, InviteDial, InvitePolicy, InviteRequest};
use yolo_hub_federation_settings::{ExportPolicy, Peer, PeerAddress};
use yolo_hub_identity::HubId;
use yolo_hub_management_protocol::{ConfigurationChange, ConfigurationSnapshot};
use yolo_hub_routing::Right;

pub fn invite_request(
    snapshot: &ConfigurationSnapshot,
    args: &InvitePeer,
) -> Result<InviteRequest> {
    ensure!(!(args.no_export && !args.import_peer), INVITE_POLICY_EMPTY);
    ensure!(
        (MIN_INVITE_MINUTES..=MAX_INVITE_MINUTES).contains(&args.ttl_minutes),
        INVITE_TTL_INVALID
    );
    let listener = snapshot.listen_address.filter(|_| snapshot.applied);
    let port = args
        .port
        .or_else(|| listener.and_then(|address| NonZeroU16::new(address.port())));
    let host = match (&args.host, listener) {
        (Some(host), _) => host.clone(),
        (None, Some(address)) if !address.ip().is_unspecified() => address.ip().to_string(),
        (None, Some(address)) => {
            anyhow::bail!("{INVITE_HOST_REQUIRED} (current listener: {address})")
        }
        (None, None) => anyhow::bail!(INVITE_LISTENER_MISSING),
    };
    let Some(port) = port else {
        anyhow::bail!(INVITE_PORT_REQUIRED)
    };
    Ok(InviteRequest {
        dial: InviteDial { host, port },
        lifetime_ms: args.ttl_minutes.saturating_mul(MINUTE_MS),
        policy: InvitePolicy {
            export_local: !args.no_export,
            import_peer: args.import_peer,
        },
    })
}

pub fn listen_change(
    snapshot: &ConfigurationSnapshot,
    address: SocketAddr,
) -> Result<ConfigurationChange> {
    let mut configuration = snapshot.state.configuration.clone();
    configuration.listen = Some(address);
    configuration.validate(snapshot.state.identity)?;
    Ok(ConfigurationChange {
        request_id: Uuid::new_v4(),
        expected_revision: snapshot.state.revision,
        configuration,
    })
}

pub fn join_change(
    snapshot: &ConfigurationSnapshot,
    claims: &InviteClaims,
) -> Result<Option<ConfigurationChange>> {
    let local = snapshot.state.identity;
    ensure!(
        claims.issuer.hub() != local.hub() && claims.issuer.key() != local.key(),
        PEER_IS_LOCAL
    );
    let consume = claims.policy.export_local;
    let exports = if claims.policy.import_peer {
        vec![ExportPolicy {
            origin: local.hub(),
            rights: [Right::Observe].into_iter().collect(),
            remaining_reexports: 0,
        }]
    } else {
        Vec::new()
    };
    let dial = PeerAddress::tcp(claims.dial.host.clone(), claims.dial.port);
    if let Some(peer) = snapshot.state.configuration.peer(claims.issuer.hub()) {
        let same = peer.identity == claims.issuer
            && peer.consume == consume
            && peer.dial.as_ref() == Some(&dial)
            && peer.exports == exports;
        return if same {
            Ok(None)
        } else {
            anyhow::bail!(PEER_ALREADY_CONFIGURED)
        };
    }
    let mut configuration = snapshot.state.configuration.clone();
    configuration.peers.push(Peer {
        identity: claims.issuer,
        dial: Some(dial),
        consume,
        exports,
    });
    configuration.validate(local)?;
    Ok(Some(ConfigurationChange {
        request_id: Uuid::new_v4(),
        expected_revision: snapshot.state.revision,
        configuration,
    }))
}

pub fn add_change(snapshot: &ConfigurationSnapshot, args: &AddPeer) -> Result<ConfigurationChange> {
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
    let dial = match (&args.host, args.port) {
        (Some(host), Some(port)) => Some(PeerAddress::tcp(host.clone(), port)),
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
    configuration.peers.push(Peer {
        identity,
        dial,
        consume: !args.no_import,
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
