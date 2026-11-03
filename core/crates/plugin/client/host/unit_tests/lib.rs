use yolo_builtin_catalog::{TUI_ID, tui_api};
use yolo_builtin_client_contract::{BuiltinClientDescriptor, Executable, ProcessEntrypoint};
use yolo_builtin_client_host::{BuiltinClientHost, ClientHostError};
use yolo_hub_rpc_protocol::{HUB_PROTOCOL_VERSION, Hello, Method};
use yolo_plugin_api::{ClientApiContract, ClientCapability, Platform};

fn descriptor(id: &str, api: ClientApiContract) -> BuiltinClientDescriptor {
    BuiltinClientDescriptor::new(
        id,
        Platform::Unix,
        ProcessEntrypoint {
            platform: Platform::Unix,
            executable: Executable::YoloImage,
            arguments: Vec::new(),
        },
        api,
    )
    .unwrap()
}

fn hello(methods: Vec<Method>) -> Hello {
    Hello {
        hub_id: yolo_hub_identity::HubId::new(),
        version: HUB_PROTOCOL_VERSION,
        methods,
    }
}

#[test]
fn duplicate_and_unknown_clients_are_rejected() {
    let mut host = BuiltinClientHost::default();
    let first = descriptor("yolo.tui", tui_api());
    host.register(first.clone()).unwrap();
    assert_eq!(host.register(first), Err(ClientHostError::DuplicateClient));
    assert_eq!(
        host.negotiate("missing", &hello(tui_api().required_methods)),
        Err(ClientHostError::UnknownClient)
    );
}

#[test]
fn platform_admission_is_explicit() {
    let linux = BuiltinClientDescriptor::new(
        "yolo.linux",
        Platform::Linux,
        ProcessEntrypoint {
            platform: Platform::Linux,
            executable: Executable::YoloImage,
            arguments: Vec::new(),
        },
        tui_api(),
    )
    .unwrap();
    let host = BuiltinClientHost::builtin([linux]).unwrap();
    assert!(
        host.start_on(
            "yolo.linux",
            Platform::Linux,
            &hello(tui_api().required_methods)
        )
        .is_ok()
    );
    assert_eq!(
        host.start_on(
            "yolo.linux",
            Platform::Macos,
            &hello(tui_api().required_methods)
        ),
        Err(ClientHostError::UnsupportedPlatform(Platform::Macos))
    );
}

#[test]
fn startup_rejects_missing_required_methods() {
    let host = BuiltinClientHost::builtin([descriptor(TUI_ID, tui_api())]).unwrap();
    assert_eq!(
        host.start(TUI_ID, &hello(vec![Method::Hello])),
        Err(ClientHostError::StartupUnsupported)
    );
}

#[test]
fn feature_projection_does_not_imply_writer_authority() {
    let host = BuiltinClientHost::builtin([descriptor(TUI_ID, tui_api())]).unwrap();
    let capabilities = host
        .start(TUI_ID, &hello(tui_api().required_methods))
        .unwrap()
        .capabilities;
    assert!(capabilities.supports(ClientCapability::Layouts));
    assert!(!capabilities.supports(ClientCapability::TerminalControl));
}
