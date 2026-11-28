use super::*;
use crate::protocol::method;
use api_matrix_support::*;
use terminal_api_matrix_assertions::{TerminalExpectations, assert_terminal_requests};

const TEST_CONTROLLER: &str = "controller-instance";
const TEST_HOLDER: &str = "terminal-api-matrix";
const TEST_SIGNAL: &str = "INT";
const TEST_INPUT: &[u8] = b"terminal input";
const TEST_TTL_MS: u64 = 101;
const TEST_COLUMNS: u16 = 103;
const TEST_ROWS: u16 = 107;
const TERMINAL_METHODS: &[&str] = &[
    method::ACQUIRE_WRITER,
    method::ACQUIRE_WRITER,
    method::TAKEOVER_WRITER,
    method::TAKEOVER_WRITER,
    method::RENEW_WRITER,
    method::RELEASE_WRITER,
    method::GET_WRITER_LEASE,
    method::TERMINAL_INPUT,
    method::TERMINAL_INPUT,
    method::RESIZE_TERMINAL,
    method::SIGNAL_SESSION,
    method::GET_SESSION,
    method::GET_SNAPSHOT,
];

#[tokio::test]
async fn every_terminal_client_api_uses_all_fenced_typed_parameters() {
    let server = MatrixServer::start(TERMINAL_METHODS).await;
    let client = Client::new(server.socket.clone());
    let target = test_target();
    let capability = WriterCapability::generate();
    let basic = server
        .rpc(client.acquire_writer(target, TEST_HOLDER, Some(TEST_TTL_MS)))
        .await
        .unwrap();
    server
        .rpc(client.acquire_writer_for_instance(
            target,
            TEST_HOLDER,
            Some(TEST_CONTROLLER.into()),
            capability.clone(),
            Some(TEST_TTL_MS),
        ))
        .await
        .unwrap();
    let takeover = server
        .rpc(client.take_over_writer(target, TEST_HOLDER, Some(TEST_TTL_MS)))
        .await
        .unwrap();
    server
        .rpc(client.take_over_writer_for_instance(
            target,
            TEST_HOLDER,
            Some(TEST_CONTROLLER.into()),
            Some(basic.generation),
            capability.clone(),
            Some(TEST_TTL_MS),
        ))
        .await
        .unwrap();
    server
        .rpc(client.renew_writer(&basic, target, Some(TEST_TTL_MS)))
        .await
        .unwrap();
    server
        .rpc(client.release_writer(&basic, target))
        .await
        .unwrap();
    assert!(
        server
            .rpc(client.get_writer_lease(target))
            .await
            .unwrap()
            .is_some()
    );
    let generated_input = server
        .rpc(client.input(
            target,
            basic.generation,
            &basic.writer_capability,
            TEST_INPUT,
        ))
        .await
        .unwrap();
    let explicit_input = InputRequest {
        terminal_id: target.terminal_id,
        runtime_epoch: target.runtime_epoch,
        generation: basic.generation,
        writer_capability: basic.writer_capability.clone(),
        input_id: test_input_id(),
        data: TEST_INPUT.to_vec(),
    };
    let explicit_result = server
        .rpc(client.input_with_id(&explicit_input))
        .await
        .unwrap();
    assert_eq!(explicit_result.input_id, explicit_input.input_id);
    server
        .rpc(client.resize(
            target,
            basic.generation,
            &basic.writer_capability,
            TEST_COLUMNS,
            TEST_ROWS,
        ))
        .await
        .unwrap();
    server
        .rpc(client.signal(
            target,
            basic.generation,
            &basic.writer_capability,
            TEST_SIGNAL,
        ))
        .await
        .unwrap();
    let snapshot = server
        .rpc(client.snapshot(target.terminal_id))
        .await
        .unwrap();
    assert_eq!(snapshot.runtime_epoch, target.runtime_epoch);

    assert_terminal_requests(
        server.finish().await,
        TerminalExpectations {
            target,
            holder: TEST_HOLDER,
            controller: TEST_CONTROLLER,
            ttl_ms: TEST_TTL_MS,
            explicit_capability: &capability,
            basic_grant: &basic,
            takeover_grant: &takeover,
            generated_input_id: generated_input.input_id,
            explicit_input: &explicit_input,
            input: TEST_INPUT,
            cols: TEST_COLUMNS,
            rows: TEST_ROWS,
            signal: TEST_SIGNAL,
        },
    );
}
