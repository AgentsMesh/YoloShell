use support::*;
use yolo_hub_delegation_test_support as support;
use yolo_hub_federation_settings::*;
mod address;
mod address_constants;
mod agent_control;
mod file_roots;

#[test]
fn default_local_configuration_and_explicit_directed_peer_roundtrip() {
    let (local, remote) = (Identity::new(), Identity::new());
    Configuration::default().validate(local.identity).unwrap();
    let configuration = Configuration {
        listen: None,
        trusted_origins: vec![],
        file_roots: vec![],
        peers: vec![Peer {
            identity: remote.identity,
            dial: None,
            consume: true,
            exports: vec![ExportPolicy {
                origin: local.identity.hub(),
                rights: rights(),
                remaining_reexports: REEXPORT_DEPTH,
            }],
        }],
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
        consume: true,
        exports: vec![],
    };
    let mut configuration = Configuration {
        listen: None,
        trusted_origins: vec![],
        file_roots: vec![],
        peers: vec![peer.clone(), peer],
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
