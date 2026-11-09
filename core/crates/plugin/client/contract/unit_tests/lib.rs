use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{HUB_PROTOCOL_VERSION, Hello, Method};
use yolo_plugin_api::{ClientApiContract, ClientCapability, Platform};
const INCOMPATIBLE_PROTOCOL_VERSION_OFFSET: u16 = 1;

use yolo_builtin_client_contract::{
    BuiltinClientDescriptor, Executable, ProcessEntrypoint, negotiate,
};

fn hello(methods: Vec<Method>) -> Hello {
    Hello {
        hub_id: HubId::new(),
        version: HUB_PROTOCOL_VERSION,
        methods,
    }
}

#[test]
fn negotiation_reports_startup_and_feature_gaps_separately() {
    let api = ClientApiContract::new(
        HUB_PROTOCOL_VERSION,
        [Method::Hello, Method::List],
        [],
        [yolo_plugin_api::ClientFeatureRequirement::new(
            ClientCapability::Layouts,
            [Method::ListLayouts],
        )],
    );
    let result = negotiate(&api, &hello(vec![Method::Hello, Method::List]));
    assert!(result.can_start());
    assert!(result.enabled_features.is_empty());
    assert_eq!(result.disabled_features.len(), 1);
}

#[test]
fn protocol_mismatch_never_enables_features() {
    let api = ClientApiContract::new(
        HUB_PROTOCOL_VERSION,
        [Method::Hello],
        [],
        [yolo_plugin_api::ClientFeatureRequirement::new(
            ClientCapability::Layouts,
            [Method::ListLayouts],
        )],
    );
    let mut hello = hello(vec![Method::Hello, Method::ListLayouts]);
    hello.version = HUB_PROTOCOL_VERSION.saturating_add(INCOMPATIBLE_PROTOCOL_VERSION_OFFSET);
    let result = negotiate(&api, &hello);
    assert!(!result.can_start());
    assert!(result.enabled_features.is_empty());
}

#[test]
fn descriptor_rejects_platform_mismatch_and_undeclared_features() {
    let api = ClientApiContract::new(
        HUB_PROTOCOL_VERSION,
        [Method::Hello],
        [],
        [yolo_plugin_api::ClientFeatureRequirement::new(
            ClientCapability::Layouts,
            [Method::ListLayouts],
        )],
    );
    let entry = ProcessEntrypoint {
        platform: Platform::Linux,
        executable: Executable::YoloImage,
        arguments: Vec::new(),
    };
    assert!(
        BuiltinClientDescriptor::new("yolo.tui", Platform::Macos, entry.clone(), api.clone())
            .is_err()
    );
    assert!(BuiltinClientDescriptor::new("yolo.tui", Platform::Linux, entry, api).is_err());
}

#[test]
fn valid_tui_image_descriptor_is_admitted() {
    let api = ClientApiContract::new(HUB_PROTOCOL_VERSION, [Method::Hello], [], []);
    let entry = ProcessEntrypoint {
        platform: Platform::Unix,
        executable: Executable::YoloImage,
        arguments: Vec::new(),
    };
    assert!(BuiltinClientDescriptor::new("yolo.tui", Platform::Unix, entry, api).is_ok());
}
