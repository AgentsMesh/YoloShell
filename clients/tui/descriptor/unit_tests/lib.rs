use yolo_tui_descriptor::{TUI_ID, tui_api, tui_descriptor};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{HUB_PROTOCOL_VERSION, Hello, Method};
use yolo_plugin_api::ClientCapability;
const INCOMPATIBLE_PROTOCOL_VERSION_OFFSET: u16 = 1;

use yolo_builtin_client_contract::{BuiltinClientDescriptor, negotiate};

#[test]
fn tui_descriptor_has_the_expected_identity() {
    let descriptor = tui_descriptor();
    assert_eq!(descriptor.id(), TUI_ID);
    assert_eq!(descriptor.platform(), yolo_plugin_api::Platform::Unix);
    assert!(descriptor.entrypoint().arguments.is_empty());
}

#[test]
fn tui_startup_viewing_does_not_require_writer_or_lifecycle_permissions() {
    let api = tui_api();
    assert_eq!(api.protocol_version, HUB_PROTOCOL_VERSION);
    assert!(!api.required_methods.contains(&Method::Create));
    assert!(!api.required_methods.contains(&Method::Input));
    assert!(api.required_methods.contains(&Method::GetLayout));
}

#[test]
fn tui_negotiation_projects_optional_features() {
    let api = tui_api();
    let hello = Hello {
        hub_id: HubId::new(),
        version: HUB_PROTOCOL_VERSION,
        methods: api.required_methods.clone(),
    };
    let result = negotiate(&api, &hello);
    assert!(result.can_start());
    assert!(result.supports(ClientCapability::Layouts));
    assert!(!result.supports(ClientCapability::TerminalControl));
    assert!(!result.supports(ClientCapability::TerminalLifecycle));
}

#[test]
fn protocol_mismatch_never_enables_features() {
    let api = tui_api();
    let hello = Hello {
        hub_id: HubId::new(),
        version: HUB_PROTOCOL_VERSION.saturating_add(INCOMPATIBLE_PROTOCOL_VERSION_OFFSET),
        methods: api.required_methods.clone(),
    };
    let result = negotiate(&api, &hello);
    assert!(!result.can_start());
    assert!(result.enabled_features.is_empty());
}

#[test]
fn descriptor_rejects_entrypoint_platform_mismatch() {
    let result = BuiltinClientDescriptor::new(
        TUI_ID,
        yolo_plugin_api::Platform::Macos,
        yolo_builtin_client_contract::ProcessEntrypoint {
            platform: yolo_plugin_api::Platform::Unix,
            executable: yolo_builtin_client_contract::Executable::YoloImage,
            arguments: Vec::new(),
        },
        tui_api(),
    );
    assert!(result.is_err());
}
