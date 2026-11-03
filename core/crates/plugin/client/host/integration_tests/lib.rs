use yolo_builtin_catalog::{TUI_ID, builtin_client_descriptors};
use yolo_builtin_client_host::{BuiltinClientHost, ClientHostError};
use yolo_hub_rpc_protocol::{HUB_PROTOCOL_VERSION, Hello, Method};

fn hello(methods: Vec<Method>) -> Hello {
    Hello {
        hub_id: yolo_hub_identity::HubId::new(),
        version: HUB_PROTOCOL_VERSION,
        methods,
    }
}

#[test]
fn bundled_tui_descriptor_is_admitted() {
    let host = BuiltinClientHost::builtin(builtin_client_descriptors()).unwrap();
    let result = host
        .start(
            TUI_ID,
            &hello(yolo_builtin_catalog::tui_api().required_methods),
        )
        .unwrap();
    assert_eq!(result.client_id, TUI_ID);
}

#[test]
fn unsupported_startup_is_reported_without_side_effects() {
    let host = BuiltinClientHost::builtin(builtin_client_descriptors()).unwrap();
    assert_eq!(
        host.start(TUI_ID, &hello(vec![Method::Hello])),
        Err(ClientHostError::StartupUnsupported)
    );
}
