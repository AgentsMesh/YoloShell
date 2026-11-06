use support::*;
use yolo_hub_delegation_test_support as support;
use yolo_hub_federation_settings::*;
mod address;
mod address_constants;
mod agent_control;
mod file_roots;

#[test]
fn ssh_peers_share_the_local_origin_by_default() {
    let local = Identity::new();
    let remote = Identity::new();
    let configuration = Configuration::default();
    assert!(!configuration.ssh_peers.require_registration);
    assert!(configuration.ssh_peers.share_local);
    let policy = configuration
        .shared_export(local.identity.hub(), local.identity.hub())
        .unwrap();
    assert_eq!(policy.origin, local.identity.hub());
    assert!(policy.rights.allows(&observe()));
    assert!(
        configuration
            .shared_export(remote.identity.hub(), local.identity.hub())
            .is_none()
    );
}

#[test]
fn schema_3_profile_dials_and_missing_share_policy_are_read() {
    let remote = Identity::new();
    let configuration = Configuration {
        peers: vec![Peer {
            identity: remote.identity,
            dial: Some(
                PeerAddress::profile("managed", "/run/user/1000/yolo/hub/peer.sock".into())
                    .unwrap(),
            ),
            consume: true,
            exports: vec![],
        }],
        ..Configuration::default()
    };
    let mut raw = serde_json::to_value(&configuration).unwrap();
    raw.as_object_mut().unwrap().remove(stringify!(ssh_peers));
    let configuration: Configuration = serde_json::from_value(raw).unwrap();
    assert!(matches!(
        configuration.peers[0].dial,
        Some(PeerAddress::Profile(_))
    ));
    assert!(configuration.ssh_peers.share_local);
    assert!(!configuration.ssh_peers.require_registration);
    assert!(configuration.ssh_peers.rights.allows(&observe()));
}

#[test]
fn default_local_configuration_and_explicit_directed_peer_roundtrip() {
    let (local, remote) = (Identity::new(), Identity::new());
    Configuration::default().validate(local.identity).unwrap();
    let configuration = Configuration {
        trusted_origins: vec![],
        file_roots: vec![],
        peers: vec![Peer {
            identity: remote.identity,
            dial: None,
            consume: false,
            exports: vec![ExportPolicy {
                origin: local.identity.hub(),
                rights: rights(),
                remaining_reexports: REEXPORT_DEPTH,
            }],
        }],
        ..Configuration::default()
    };
    configuration.validate(local.identity).unwrap();
    assert_eq!(configuration.pins(), vec![remote.identity]);
    assert!(
        configuration
            .export(remote.identity.hub(), local.identity.hub())
            .is_some()
    );
    let decoded: Configuration =
        serde_json::from_value(serde_json::to_value(&configuration).unwrap()).unwrap();
    assert_eq!(decoded, configuration);
}

#[test]
fn duplicate_peers_unpinned_exports_and_self_connections_are_rejected() {
    let (local, remote, unknown) = (Identity::new(), Identity::new(), Identity::new());
    let peer = Peer {
        identity: remote.identity,
        dial: None,
        consume: false,
        exports: vec![],
    };
    let mut configuration = Configuration {
        trusted_origins: vec![],
        file_roots: vec![],
        peers: vec![peer.clone(), peer],
        ..Configuration::default()
    };
    assert!(configuration.validate(local.identity).is_err());
    configuration.peers.pop();
    configuration.peers[0].exports.push(ExportPolicy {
        origin: unknown.identity.hub(),
        rights: observe(),
        remaining_reexports: 0,
    });
    assert!(configuration.validate(local.identity).is_err());
    configuration.trusted_origins.push(unknown.identity);
    configuration.validate(local.identity).unwrap();
    configuration.peers[0].identity = local.identity;
    assert!(configuration.validate(local.identity).is_err());
}

#[test]
fn legacy_listener_and_dial_are_migrated_without_weakening_the_peer_pin() {
    let (local, remote) = (Identity::new(), Identity::new());
    let configuration = Configuration {
        peers: vec![Peer {
            identity: remote.identity,
            dial: Some(
                PeerAddress::profile(
                    "remote",
                    "/run/user/1000/yolo/hub/peer.sock".into(),
                )
                .unwrap(),
            ),
            consume: true,
            exports: vec![ExportPolicy {
                origin: local.identity.hub(),
                rights: rights(),
                remaining_reexports: REEXPORT_DEPTH,
            }],
        }],
        ..Configuration::default()
    };
    for dial in [
        serde_json::json!({
            "host": "127.0.0.1",
            "port": 7400,
            "transport": {"kind": "ssh", "profile": serde_json::to_value(
                SshProfileRef::new("remote", "a".repeat(64)).unwrap()
            ).unwrap()},
        }),
        serde_json::json!({
            "host": "192.0.2.1",
            "port": 7400,
            "transport": {"kind": "tcp"},
        }),
    ] {
        let mut value = serde_json::to_value(&configuration).unwrap();
        value["listen"] = serde_json::json!("0.0.0.0:7400");
        value["peers"][0]["dial"] = dial;
        let decoded: Configuration = serde_json::from_value(value).unwrap();
        assert_eq!(decoded.peers[0].identity, remote.identity);
        assert!(decoded.peers[0].dial.is_none());
        assert!(!decoded.peers[0].consume);
        assert_eq!(decoded.peers[0].exports, configuration.peers[0].exports);
    }
}

#[test]
fn consuming_a_peer_requires_a_dial() {
    let (local, remote) = (Identity::new(), Identity::new());
    let configuration = Configuration {
        peers: vec![Peer {
            identity: remote.identity,
            dial: None,
            consume: true,
            exports: vec![],
        }],
        ..Configuration::default()
    };
    assert!(configuration.validate(local.identity).is_err());
}
