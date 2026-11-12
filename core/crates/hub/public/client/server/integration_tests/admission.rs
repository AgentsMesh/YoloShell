use super::support::*;
use std::sync::atomic::Ordering;
use yolo_hub_client::Client;
use yolo_hub_jsonrpc_protocol::Call as Request;
use yolo_hub_jsonrpc_transport::Framed;
use yolo_hub_rpc_protocol::{Command, Method, Reply};
use yolo_hub_terminal_protocol::PublicError;

#[tokio::test]
async fn real_ssh_client_uses_public_rpc_with_scoped_inventory_on_independent_channels() {
    let fixture = Fixture::start().await;
    let connection = fixture.connection().await;
    let mut first = Client::open(
        connection.open_channel(deadline()).await.unwrap(),
        fixture.state.hub_id(),
        deadline(),
    )
    .await
    .unwrap();
    let mut second = Client::open(
        connection.open_channel(deadline()).await.unwrap(),
        fixture.state.hub_id(),
        deadline(),
    )
    .await
    .unwrap();
    assert!(first.hello().methods.contains(&Method::List));
    assert!(!first.hello().methods.contains(&Method::Configure));
    assert!(!first.hello().methods.contains(&Method::Lease));
    let Reply::Terminals { terminals, next } = first
        .call(
            fixture.state.hub_id(),
            Command::List {
                include_archived: false,
                after: None,
            },
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(terminals.len(), 1);
    assert_eq!(
        terminals[0].fields().address.hub_id,
        fixture.backend.allowed
    );
    assert_eq!(next, None);
    first.close(deadline()).await.unwrap();
    assert!(matches!(
        second
            .call(fixture.state.hub_id(), Command::Hello {}, deadline())
            .await
            .unwrap(),
        Reply::Hello { .. }
    ));
    connection.close(deadline()).await.unwrap();
    fixture.finish().await;
}

#[tokio::test]
async fn same_uid_plain_public_rpc_cannot_bypass_client_authentication() {
    let fixture = Fixture::start().await;
    let stream = tokio::net::UnixStream::connect(&fixture.socket)
        .await
        .unwrap();
    let mut frame = Framed::new(stream);
    let request = Request::new(fixture.state.hub_id(), Command::Hello {}).unwrap();
    frame.send(&request, deadline()).await.unwrap();
    assert!(
        frame
            .receive::<yolo_hub_jsonrpc_protocol::Answer>(deadline())
            .await
            .is_err()
    );
    assert_eq!(fixture.backend.calls.load(Ordering::SeqCst), 0);
    fixture.finish().await;
}

#[tokio::test]
async fn callers_cannot_bypass_advertised_methods_or_origin_scope_with_raw_requests() {
    let fixture = Fixture::start().await;
    let connection = fixture.connection().await;
    let client = Client::open(
        connection.open_channel(deadline()).await.unwrap(),
        fixture.state.hub_id(),
        deadline(),
    )
    .await
    .unwrap();
    let mut channel = client.into_channel().unwrap();
    let before = fixture.backend.calls.load(Ordering::SeqCst);
    for (origin, command) in [
        (fixture.state.hub_id(), Command::GetConfiguration {}),
        (
            fixture.backend.hidden,
            Command::List {
                include_archived: false,
                after: None,
            },
        ),
        (
            fixture.backend.allowed,
            Command::Create {
                request: yolo_hub_terminal_protocol::CreateShellRequest::new(
                    yolo_hub_terminal_protocol::CreateShellFields {
                        schema_version: yolo_hub_terminal_protocol::TERMINAL_PROTOCOL_SCHEMA_VERSION,
                        request_id: uuid::Uuid::new_v4(),
                        creation_token: uuid::Uuid::new_v4(),
                        name: None,
                        cwd: SESSION_DIRECTORY.into(),
                        size: Default::default(),
                    },
                )
                .unwrap(),
            },
        ),
    ] {
        let request = Request::new(origin, command).unwrap();
        channel.send(&request, deadline()).await.unwrap();
        let yolo_hub_jsonrpc_transport::Incoming::Message(response) = channel
            .receive::<yolo_hub_jsonrpc_protocol::Answer>(deadline())
            .await
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(
            response.reply(&request).unwrap(),
            Reply::Error {
                code: PublicError::Unauthorized
            }
        );
    }
    assert_eq!(fixture.backend.calls.load(Ordering::SeqCst), before);
    drop(channel);
    connection.close(deadline()).await.unwrap();
    fixture.finish().await;
}
