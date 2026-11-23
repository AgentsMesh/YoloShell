use uuid::Uuid;
use yolo_agent_provider_contract::LaunchPolicies;
use yolo_hub_agent_protocol::{AGENT_PROTOCOL_SCHEMA_VERSION, AgentStartFields, AgentStartRequest};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::Reply;
use yolo_runtime_rpc_testing::Peer;
use yolo_terminal_model::{
    CreateTerminal, CreateTerminalResult, CreationQueryOutcome, CreationQueryResult,
};
use yolo_wire::{ErrorBody, ResponseEnvelope, method};

#[tokio::test]
async fn a_retry_recovers_the_original_startup_receipt_without_launching_the_old_command() {
    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join("host.sock");
    let listener = tokio::net::UnixListener::bind(&socket).unwrap();
    let client = yolo_client::Client::new(socket);
    let hub = HubId::new();
    let start_token = Uuid::new_v4();
    let terminal_id = Uuid::new_v4();
    let request = AgentStartRequest::new(AgentStartFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        start_token,
        provider_id: "codex".into(),
        name: None,
        cwd: "/tmp".into(),
        size: Default::default(),
        workspace_id: None,
        parent: None,
    })
    .unwrap();
    let mut policies = LaunchPolicies::none();
    policies.insert_versioned(
        yolo_agent_contract::AgentKind::Codex,
        "current",
        Some("previous"),
    );
    let server = tokio::spawn(async move {
        for (expected_method, input) in [
            (method::CREATE_SESSION, b"current\r".as_slice()),
            (method::QUERY_CREATION, b"current\r"),
            (method::QUERY_CREATION, b"previous\r"),
        ] {
            let (stream, _) = listener.accept().await.unwrap();
            let mut peer = Peer::new(stream);
            let call = peer.request().await.unwrap();
            assert_eq!(call.method, expected_method);
            let creation: CreateTerminal = call.decode_payload().unwrap();
            assert_eq!(creation.creation_token, start_token);
            assert_eq!(creation.initial_input.as_deref(), Some(input));
            let answer = if input == b"previous\r" {
                let creation = CreateTerminalResult {
                    terminal_id,
                    creation_capability: Uuid::new_v4(),
                };
                ResponseEnvelope::success(
                    &call,
                    &CreationQueryResult {
                        creation_token: start_token,
                        outcome: CreationQueryOutcome::Finalized { creation },
                    },
                )
                .unwrap()
            } else {
                ResponseEnvelope::failure(
                    &call,
                    ErrorBody::conflict("different request fingerprint"),
                )
            };
            peer.respond(&answer).await.unwrap();
        }
    });
    let reply = crate::agent_start(hub, &client, Some(&policies), None, None, &request)
        .await
        .unwrap();
    let Reply::AgentStart { start } = reply else {
        panic!("expected agent start")
    };
    assert_eq!(start.address.terminal_id.as_uuid(), terminal_id);
    server.await.unwrap();
}
