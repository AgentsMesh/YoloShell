use yolo_hub_rpc_protocol::{HUB_PROTOCOL_VERSION, Hello, Method};
use yolo_plugin_api::{
    ClientApiContract, ClientCapability, ClientFeatureRequirement, Platform, valid_action_id,
};

#[test]
fn platform_matching_distinguishes_unix_from_specific_hosts() {
    assert!(Platform::Unix.runs_on(Platform::Linux));
    assert!(Platform::Unix.runs_on(Platform::Macos));
    assert!(!Platform::Macos.runs_on(Platform::Linux));
    assert!(Platform::Linux.runs_on(Platform::Linux));
}

#[test]
fn action_ids_use_one_lowercase_vocabulary() {
    for value in ["status", "stop-all", "restart_1"] {
        assert!(valid_action_id(value), "{value}");
    }
    for value in ["", "-bad", "Bad", "with space", "a/b"] {
        assert!(!valid_action_id(value), "{value}");
    }
}

#[test]
fn api_contract_checks_required_methods_and_features() {
    let api = ClientApiContract::new(
        HUB_PROTOCOL_VERSION,
        [Method::Hello, Method::List],
        [],
        [ClientFeatureRequirement::new(
            ClientCapability::Layouts,
            [Method::ListLayouts],
        )],
    );
    let hello = Hello {
        hub_id: yolo_hub_identity::HubId::new(),
        version: HUB_PROTOCOL_VERSION,
        methods: vec![Method::Hello, Method::List],
    };
    assert!(api.permits_connection(&hello));
    assert_eq!(api.enabled_features(&hello), Vec::<ClientCapability>::new());
}
