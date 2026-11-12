use super::{channel_peer::*, constants::*};
use crate::{Options, client_capabilities};
use tokio::net::UnixListener;
use yolo_tui_descriptor::{TUI_ID, tui_api};
use yolo_builtin_client_contract::ClientCapability;
use yolo_hub_client::HubEndpoint;
use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_transport::Framed;
use yolo_hub_rpc_protocol::{HUB_PROTOCOL_VERSION, Hello, Method, Reply};

#[tokio::test]
async fn startup_rejects_missing_live_methods_before_ui_side_effects() {
    let result = open(vec![Method::Hello]).await;
    assert!(result.is_err());
    let message = result.unwrap_err().to_string();
    assert!(message.contains("startup requirements unavailable"));
    assert!(message.contains("Attach"));
}

#[tokio::test]
async fn startup_accepts_required_methods_and_projects_optional_absence() {
    let profile = tui_api();
    let capabilities = open(profile.required_methods).await.unwrap();
    assert!(capabilities.can_start());
    assert!(!capabilities.supports(ClientCapability::TerminalControl));
    assert!(!capabilities.supports(ClientCapability::TerminalLifecycle));
    assert!(!capabilities.supports(ClientCapability::Conversations));
    assert!(!capabilities.supports(ClientCapability::HubManagement));
}

async fn open(
    methods: Vec<Method>,
) -> anyhow::Result<yolo_builtin_client_contract::CapabilityNegotiation> {
    let root = tempfile::tempdir_in(NETWORK_TEST_ROOT).unwrap();
    let socket = root.path().join(NETWORK_TEST_SOCKET);
    let listener = UnixListener::bind(&socket).unwrap();
    let hub = HubId::new();
    let options = Options {
        endpoint: HubEndpoint::local(socket),
        hub,
        layout: None,
    };
    let peer = async {
        let (stream, _) = listener.accept().await.unwrap();
        let mut peer = Framed::new(stream);
        let request = receive(&mut peer).await;
        respond(
            &mut peer,
            &request,
            Reply::Hello {
                hello: Hello {
                    hub_id: hub,
                    version: HUB_PROTOCOL_VERSION,
                    methods,
                },
            },
        )
        .await;
    };
    let (result, ()) = tokio::join!(client_capabilities::open(&options), peer);
    result
}
