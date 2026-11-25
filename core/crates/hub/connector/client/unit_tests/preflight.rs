use super::{
    fixtures::{constants::*, *},
    server::*,
};
use crate::ClientError;
use tokio::time::Instant;
use yolo_hub_connector_protocol::{ConnectorError, Reply};
use yolo_hub_jsonrpc_transport::TransportError;

#[tokio::test]
async fn invalid_request_origin_and_expired_deadline_do_not_write_or_retire_a_channel() {
    let (mut client, mut server) = connected().await;
    let mut invalid = activation_request();
    invalid.request_id = uuid::Uuid::nil();
    assert!(matches!(
        client.call(invalid, deadline()).await,
        Err(ClientError::Validation(ConnectorError::InvalidRequest))
    ));
    let mut invalid = activation_request();
    invalid.version += SCHEMA_STEP;
    assert!(matches!(
        client.call(invalid, deadline()).await,
        Err(ClientError::Validation(ConnectorError::UnsupportedSchema))
    ));
    let mut wrong = activation_request();
    wrong.hub = other_hub();
    assert!(matches!(
        client.call(wrong, deadline()).await,
        Err(ClientError::Validation(ConnectorError::Unauthorized))
    ));
    assert!(matches!(
        client.call(activation_request(), Instant::now()).await,
        Err(ClientError::Transport(TransportError::Deadline))
    ));
    assert!(client.is_usable());
    let peer = async {
        let request = receive(&mut server).await;
        assert_eq!(request, hello());
        respond(&mut server, &request, Reply::Hello { hub: hub() }).await;
    };
    let (reply, ()) = tokio::join!(client.call(hello(), deadline()), peer);
    assert_eq!(reply.unwrap(), Reply::Hello { hub: hub() });
}

#[test]
fn diagnostics_and_error_chains_cannot_expose_credential_or_decoder_bytes() {
    use std::error::Error;
    let hidden = || TransportError::Io(std::io::Error::other(CREDENTIAL));
    for (error, unknown) in [
        (ClientError::Transport(hidden()), false),
        (ClientError::OutcomeUnknown(hidden()), true),
        (
            ClientError::OutcomeUnknownResponse(ConnectorError::Conflict),
            true,
        ),
        (
            ClientError::InvalidResponse(ConnectorError::Conflict),
            false,
        ),
        (ClientError::Remote(ConnectorError::OutcomeUnknown), true),
        (ClientError::Remote(ConnectorError::Unauthorized), false),
        (
            ClientError::Validation(ConnectorError::InvalidRequest),
            false,
        ),
        (ClientError::Poisoned, false),
        (ClientError::WrongLocalUser, false),
    ] {
        assert_eq!(error.outcome_is_unknown(), unknown);
        assert!(!error.to_string().contains(CREDENTIAL));
        assert!(!format!("{error:?}").contains(CREDENTIAL));
        assert!(!error.to_string().is_empty());
        assert!(error.source().is_none());
    }
}
