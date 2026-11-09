use super::*;
use crate::protocol::{error_code, method};
use crate::relay::session_test_support::{
    GENERATION, closed_resize_channel, host_status, options, other_lease, owned_prefix,
    read_only_prefix, running_session,
};
use crate::relay::test_support::{FakeHost, Reply};
use crate::worker::{InputRequest, ResizeRequest};
use std::io::Cursor;

const INPUT: &[u8] = b"input survives rejected reconciliation failures";
const RECOVERED_GENERATION: u64 = GENERATION + 2;
const INPUT_ATTEMPTS: usize = 2;
const DELIVERED_INPUT_ATTEMPTS: usize = 1;
const RESIZE_ATTEMPTS: usize = 4;
const TEST_COLUMNS: u16 = 109;
const TEST_ROWS: u16 = 43;

#[tokio::test]
async fn rejected_input_survives_reconcile_disconnect_and_retryable_error() {
    let terminal_id = Uuid::new_v4();
    let mut replies = owned_prefix(terminal_id);
    replies.extend([
        Reply::error(crate::relay::constants::error_code::NOT_HOLDER),
        Reply::Disconnect,
        Reply::value(&host_status()),
        Reply::value(&running_session(terminal_id)),
        Reply::retryable_error(error_code::WORKER_UNREACHABLE),
        Reply::value(&host_status()),
        Reply::value(&running_session(terminal_id)),
        Reply::value(&Some(other_lease())),
        Reply::Lease(RECOVERED_GENERATION),
        Reply::InputDelivered,
        Reply::value(&()),
    ]);
    let host = FakeHost::start(replies).await;
    let exit = relay_session(
        RelayRuntime::existing(host.client()),
        options(terminal_id),
        Cursor::new(INPUT),
        tokio::io::sink(),
        closed_resize_channel(),
    )
    .await
    .unwrap();
    assert_eq!(exit, RelayExit::InputClosed);

    let inputs = host
        .requests()
        .into_iter()
        .filter(|request| request.method == method::TERMINAL_INPUT)
        .map(|request| request.decode_payload::<InputRequest>().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(inputs.len(), INPUT_ATTEMPTS);
    let first = inputs.first().unwrap();
    let recovered = inputs.last().unwrap();
    assert_eq!(first.input_id, recovered.input_id);
    assert!(inputs.iter().all(|input| input.data == INPUT));
    assert_eq!(first.generation, GENERATION);
    assert_eq!(recovered.generation, RECOVERED_GENERATION);
}

#[tokio::test]
async fn pending_input_survives_confirmed_resize_pre_admission_failures() {
    let terminal_id = Uuid::new_v4();
    let size = TerminalSize::new(TEST_COLUMNS, TEST_ROWS).unwrap();
    let mut replies = read_only_prefix(terminal_id);
    replies.extend([
        Reply::Lease(RECOVERED_GENERATION),
        Reply::retryable_error(error_code::WORKER_UNREACHABLE),
        Reply::value(&host_status()),
        Reply::value(&running_session(terminal_id)),
        Reply::retryable_error(error_code::WORKER_UNREACHABLE),
        Reply::value(&host_status()),
        Reply::value(&running_session(terminal_id)),
        Reply::retryable_error(error_code::WORKER_UNREACHABLE),
        Reply::value(&host_status()),
        Reply::value(&running_session(terminal_id)),
        Reply::value(&()),
        Reply::InputDelivered,
        Reply::value(&()),
    ]);
    let host = FakeHost::start(replies).await;
    let exit = relay_session(
        RelayRuntime::existing(host.client()),
        options(terminal_id).initial_size(Some(size)),
        Cursor::new(INPUT),
        tokio::io::sink(),
        closed_resize_channel(),
    )
    .await
    .unwrap();
    assert_eq!(exit, RelayExit::InputClosed);

    let requests = host.requests();
    let resizes = requests
        .iter()
        .filter(|request| request.method == method::RESIZE_TERMINAL)
        .map(|request| request.decode_payload::<ResizeRequest>().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(resizes.len(), RESIZE_ATTEMPTS);
    assert!(resizes.iter().all(|request| request.cols == TEST_COLUMNS));
    assert!(resizes.iter().all(|request| request.rows == TEST_ROWS));
    assert!(
        resizes
            .iter()
            .all(|request| request.generation == RECOVERED_GENERATION)
    );
    let inputs = requests
        .iter()
        .filter(|request| request.method == method::TERMINAL_INPUT)
        .map(|request| request.decode_payload::<InputRequest>().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(inputs.len(), DELIVERED_INPUT_ATTEMPTS);
    assert_eq!(inputs[0].data, INPUT);
    assert_eq!(inputs[0].generation, RECOVERED_GENERATION);
}
