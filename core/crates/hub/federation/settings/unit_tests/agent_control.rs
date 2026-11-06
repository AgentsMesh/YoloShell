use yolo_hub_delegation_test_support::Identity;
use yolo_hub_federation_settings::{Configuration, ExportPolicy, Peer};
use yolo_hub_routing::{Right, Rights};

#[test]
fn semantic_control_export_is_explicit_and_old_terminal_exports_do_not_expand() {
    let local = Identity::new();
    let remote = Identity::new();
    for semantic in [false, true] {
        let mut rights: Rights = [Right::Observe, Right::TerminalWrite].into_iter().collect();
        if semantic {
            rights = rights.union(&[Right::AgentControl].into_iter().collect());
        }
        let configuration = Configuration {
            peers: vec![Peer {
                identity: remote.identity,
                dial: None,
                consume: false,
                exports: vec![ExportPolicy {
                    origin: local.identity.hub(),
                    rights,
                    remaining_reexports: 0,
                }],
            }],
            ..Configuration::default()
        };
        configuration.validate(local.identity).unwrap();
        let restored: Configuration =
            serde_json::from_slice(&serde_json::to_vec(&configuration).unwrap()).unwrap();
        let policy = restored
            .export(remote.identity.hub(), local.identity.hub())
            .unwrap();
        assert_eq!(
            policy
                .rights
                .allows(&[Right::AgentControl].into_iter().collect()),
            semantic
        );
        assert!(
            policy
                .rights
                .allows(&[Right::TerminalWrite].into_iter().collect())
        );
        assert!(
            !policy
                .rights
                .allows(&[Right::Lifecycle].into_iter().collect())
        );
        assert!(
            !policy
                .rights
                .allows(&[Right::FileRead].into_iter().collect())
        );
    }
}

#[test]
fn semantic_only_export_does_not_implicitly_allow_terminal_writes() {
    let local = Identity::new();
    let remote = Identity::new();
    let configuration = Configuration {
        peers: vec![Peer {
            identity: remote.identity,
            dial: None,
            consume: false,
            exports: vec![ExportPolicy {
                origin: local.identity.hub(),
                rights: [Right::Observe, Right::AgentControl].into_iter().collect(),
                remaining_reexports: 0,
            }],
        }],
        ..Configuration::default()
    };
    configuration.validate(local.identity).unwrap();
    let policy = configuration
        .export(remote.identity.hub(), local.identity.hub())
        .unwrap();
    assert!(
        !policy.rights.allows(
            &[Right::AgentControl, Right::TerminalWrite]
                .into_iter()
                .collect()
        )
    );
}
