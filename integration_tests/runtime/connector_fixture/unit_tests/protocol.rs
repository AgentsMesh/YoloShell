use crate::{FixtureError, constants::*, lines, runner::decode};
use std::error::Error;
use tokio::io::{AsyncReadExt, duplex};
use yolo_hub_connector_protocol::{
    Command, ConnectorCredential, ConnectorError, FIRST_CONNECTOR_REVISION, Reply, Request,
    Response,
};
use yolo_hub_identity::{ConnectorId, ConnectorInstanceId, HubId};

#[test]
fn driver_supplies_credentials_only_in_explicit_activation_request() {
    let hub = HubId::new();
    let credential = ConnectorCredential::generate();
    let request = Request::new(
        hub,
        Command::Activate {
            connector_id: ConnectorId::new(),
            credential,
            instance_id: ConnectorInstanceId::new(),
        },
    )
    .unwrap();
    let line = serde_json::to_vec(&request).unwrap();
    assert_eq!(decode(&line, hub).unwrap(), request);
    assert!(!format!("{request:?}").contains(&credential.as_uuid().to_string()));
    assert!(matches!(
        decode(&line, HubId::new()),
        Err(FixtureError::InvalidRequest)
    ));
}

#[test]
fn fixture_rejects_admin_actions_malformed_input_and_non_matching_origin() {
    let hub = HubId::new();
    for command in [
        Command::List {},
        Command::Revoke {
            connector_id: ConnectorId::new(),
            expected_revision: FIRST_CONNECTOR_REVISION,
        },
    ] {
        let request = Request::new(hub, command).unwrap();
        assert!(matches!(
            decode(&serde_json::to_vec(&request).unwrap(), hub),
            Err(FixtureError::InvalidRequest)
        ));
    }
    let hello = Request::new(hub, Command::Hello {}).unwrap();
    assert_eq!(
        decode(&serde_json::to_vec(&hello).unwrap(), hub).unwrap(),
        hello
    );
    assert!(matches!(
        decode(INVALID_JSON, hub),
        Err(FixtureError::InvalidRequest)
    ));
    assert!(matches!(
        decode(&vec![0; MAX_LINE_BYTES + DELIMITER_BYTES], hub),
        Err(FixtureError::TooLarge)
    ));
}

#[tokio::test]
async fn output_is_an_exact_correlated_response_line() {
    let request = Request::new(HubId::new(), Command::Hello {}).unwrap();
    let response = Response::new(
        &request,
        Reply::Error {
            code: ConnectorError::Unavailable,
        },
    );
    let (mut output, mut input) = duplex(INPUT_BUFFER_BYTES);
    lines::write(&mut output, &response).await.unwrap();
    drop(output);
    let mut bytes = Vec::new();
    input.read_to_end(&mut bytes).await.unwrap();
    assert_eq!(bytes.pop(), Some(LINE_END));
    let decoded: Response = serde_json::from_slice(&bytes).unwrap();
    decoded.validate_for(&request).unwrap();
    assert_eq!(decoded, response);
}

#[test]
fn fixture_diagnostics_are_fixed_and_never_expose_error_causes() {
    for error in [
        FixtureError::InvalidRequest,
        FixtureError::Input,
        FixtureError::Output,
        FixtureError::TooLarge,
        FixtureError::Deadline,
        FixtureError::Client(yolo_hub_connector_client::ClientError::Remote(
            ConnectorError::Unauthorized,
        )),
    ] {
        assert!(!error.to_string().is_empty());
        assert_eq!(format!("{error:?}"), error.to_string());
        assert!(error.source().is_none());
    }
}

const INVALID_JSON: &[u8] = b"{";
