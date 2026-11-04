use super::{constants::*, fixture::*};
use crate::connection;
use yolo_hub_connector_jsonrpc::{Answer, Call};
use yolo_hub_connector_protocol::{Command, Request};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_local_unix::LocalUnixStream;

#[tokio::test]
async fn missing_hello_and_foreign_destination_are_rejected_before_registry_operations() {
    for foreign in [false, true] {
        let fixture = Fixture::new();
        let (left, right) = LocalUnixStream::pair().unwrap();
        assert!(connection::same_user(&left));
        let request = if foreign {
            Request::new(yolo_hub_identity::HubId::new(), Command::Hello {}).unwrap()
        } else {
            fixture.request(Command::List {})
        };
        let request = Call::from_request(request);
        let client = async {
            let mut client = Framed::new(left);
            client.send(&request, deadline()).await.unwrap();
            assert!(!matches!(
                client.receive::<Answer>(deadline()).await,
                Ok(Incoming::Message(_))
            ));
        };
        let (result, ()) = tokio::join!(
            connection::serve(right, fixture.state.clone(), fixture.backend.clone()),
            client
        );
        assert!(result.is_err());
        assert!(fixture.backend.list_connectors().await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn wrong_schema_or_secret_bearing_malformed_frame_never_leaks_decoding_details() {
    for wrong_schema in [false, true] {
        let fixture = Fixture::new();
        let (left, right) = LocalUnixStream::pair().unwrap();
        let mut request =
            serde_json::to_value(Call::from_request(fixture.request(Command::Hello {}))).unwrap();
        if wrong_schema {
            request["jsonrpc"] = SECRET_TEXT.into();
        } else {
            request["params"][UNKNOWN_COMMAND] = SECRET_TEXT.into();
        }
        let client = async {
            let mut client = Framed::new(left);
            client.send(&request, deadline()).await.unwrap();
            let Incoming::Message(response) = client
                .receive::<yolo_hub_jsonrpc_protocol::Answer>(deadline())
                .await
                .unwrap()
            else {
                panic!()
            };
            assert!(response.error.is_some());
            assert!(
                !serde_json::to_string(&response)
                    .unwrap()
                    .contains(SECRET_TEXT)
            );
        };
        let (result, ()) = tokio::join!(
            connection::serve(right, fixture.state, fixture.backend),
            client
        );
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn dropping_a_greeted_but_unactivated_client_is_a_clean_eof() {
    let fixture = Fixture::new();
    let (left, right) = LocalUnixStream::pair().unwrap();
    let request = Call::from_request(fixture.request(Command::Hello {}));
    let client = async {
        let mut client = Framed::new(left);
        client.send(&request, deadline()).await.unwrap();
        let Incoming::Message(response) = client.receive::<Answer>(deadline()).await.unwrap()
        else {
            panic!()
        };
        response.reply(&request).unwrap();
        // Peer credentials are established before disconnect; a closed socket
        // need not retain readable credentials on every supported kernel.
    };
    let (result, ()) = tokio::join!(
        connection::serve(right, fixture.state, fixture.backend),
        client
    );
    assert!(result.is_ok());
}
