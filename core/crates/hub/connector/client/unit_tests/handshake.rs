use super::{
    fixtures::{constants::*, *},
    server::*,
};
use crate::{Client, ClientError};
use tokio::io::duplex;
use yolo_hub_connector_protocol::{ConnectorError, Reply};
use yolo_hub_jsonrpc_transport::Framed;

#[tokio::test]
async fn hello_pins_the_origin_and_explicit_close_retires_the_channel() {
    let (mut client, mut server) = connected().await;
    assert_eq!(client.hub_id(), hub());
    assert!(client.is_usable());
    client.close(deadline()).await.unwrap();
    eof(&mut server).await;
    assert!(!client.is_usable());
    assert!(matches!(
        client.close(deadline()).await,
        Err(ClientError::Poisoned)
    ));
}

#[tokio::test]
async fn handshake_rejects_foreign_identity_wrong_reply_and_known_rejection() {
    for reply in [
        Reply::Hello { hub: other_hub() },
        Reply::Enrollments {
            enrollments: vec![],
        },
        Reply::Error {
            code: ConnectorError::Unauthorized,
        },
    ] {
        let (left, right) = duplex(PIPE_CAPACITY);
        let peer = async move {
            let mut server = Framed::new(right);
            let request = receive(&mut server).await;
            respond(&mut server, &request, reply).await;
        };
        let (result, ()) = tokio::join!(Client::open(left, hub(), deadline()), peer);
        let error = result.err().unwrap();
        assert!(!error.outcome_is_unknown());
    }
}

#[test]
fn local_peer_authentication_rejects_another_uid() {
    assert!(crate::local::validate_uid(LOCAL_UID, LOCAL_UID).is_ok());
    assert!(matches!(
        crate::local::validate_uid(OTHER_UID, LOCAL_UID),
        Err(ClientError::WrongLocalUser)
    ));
}
