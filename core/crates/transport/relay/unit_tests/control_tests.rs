use super::*;
use crate::protocol::error_code::INTERNAL as INTERNAL_CODE;
use crate::protocol::method;
use crate::relay::test_support::{FakeHost, Reply};

const GENERATION: u64 = 7;
const SIGNAL_NAME: &str = "SIGINT";
const INPUT_PAYLOAD: &[u8] = b"once";
const TEST_RUNTIME_EPOCH_VALUE: u128 = 1;

fn target(terminal_id: Uuid) -> RuntimeRef {
    RuntimeRef::new(terminal_id, Uuid::from_u128(TEST_RUNTIME_EPOCH_VALUE))
}

#[tokio::test]
async fn mutations_share_one_connection_and_preserve_canonical_generation() {
    let capability = WriterCapability::generate();
    let input_id = Uuid::new_v4();
    let input_result = InputResult {
        input_id,
        duplicate: false,
        delivery: crate::protocol::delivery::DELIVERED.into(),
    };
    let host = FakeHost::start(vec![
        Reply::value(&input_result),
        Reply::value(&()),
        Reply::value(&()),
    ])
    .await;
    let id = Uuid::new_v4();
    let mut channel = ControlChannel::connect(RelayRuntime::existing(host.client()), target(id))
        .await
        .unwrap();
    assert_eq!(
        channel
            .input(GENERATION, &capability, input_id, b"input")
            .await
            .unwrap()
            .input_id,
        input_id
    );
    channel
        .resize(GENERATION, &capability, TerminalSize::new(90, 30).unwrap())
        .await
        .unwrap();
    channel
        .signal(GENERATION, &capability, SIGNAL_NAME)
        .await
        .unwrap();
    assert_eq!(
        host.methods(),
        vec![
            method::TERMINAL_INPUT,
            method::RESIZE_TERMINAL,
            method::SIGNAL_SESSION
        ]
    );
}

#[tokio::test]
async fn input_with_unknown_delivery_is_never_replayed() {
    let id = Uuid::new_v4();
    let input_id = Uuid::new_v4();
    let capability = WriterCapability::generate();
    let host = FakeHost::start(vec![Reply::Disconnect]).await;
    let mut channel = ControlChannel::connect(RelayRuntime::existing(host.client()), target(id))
        .await
        .unwrap();

    let result = channel
        .input(GENERATION, &capability, input_id, INPUT_PAYLOAD)
        .await;
    assert!(matches!(
        result,
        Err(RelayError::Client(ClientError::Protocol(
            crate::protocol::ProtocolError::OutcomeUnknown
        )))
    ));
    assert_eq!(host.methods(), vec![method::TERMINAL_INPUT]);
    let input: InputRequest = host.requests().remove(0).decode_payload().unwrap();
    assert_eq!(input.input_id, input_id);
    assert_eq!(input.data, INPUT_PAYLOAD);
}

#[tokio::test]
async fn signal_is_not_retried_when_delivery_is_unknown() {
    let capability = WriterCapability::generate();
    let host = FakeHost::start(vec![Reply::Disconnect]).await;
    let mut channel = ControlChannel::connect(
        RelayRuntime::existing(host.client()),
        target(Uuid::new_v4()),
    )
    .await
    .unwrap();

    let result = channel.signal(GENERATION, &capability, SIGNAL_NAME).await;
    assert!(matches!(
        result,
        Err(RelayError::Client(ClientError::Protocol(
            crate::protocol::ProtocolError::OutcomeUnknown
        )))
    ));
    assert_eq!(host.methods(), vec![method::SIGNAL_SESSION]);
}

#[tokio::test]
async fn remote_errors_are_not_retried_and_error_projection_is_strict() {
    let capability = WriterCapability::generate();
    let host = FakeHost::start(vec![Reply::error(INTERNAL_CODE)]).await;
    let mut channel = ControlChannel::connect(
        RelayRuntime::existing(host.client()),
        target(Uuid::new_v4()),
    )
    .await
    .unwrap();
    let error = channel
        .input(GENERATION, &capability, Uuid::new_v4(), b"rejected")
        .await
        .unwrap_err();
    assert!(client_error(&error).is_some());
    assert!(client_error(&RelayError::InvalidHolderId).is_none());
    assert_eq!(host.methods(), vec![method::TERMINAL_INPUT]);
}
