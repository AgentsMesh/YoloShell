use super::*;
use crate::protocol::{error_code, method};
use crate::relay::session_test_support::{host_status, running_session, test_runtime_epoch};
use crate::relay::test_support::{FakeHost, Reply};

const GENERATION: u64 = 41;
const INPUT: &[u8] = b"stable deduplicated input";
const INPUT_ATTEMPTS: usize = 3;

#[tokio::test]
async fn input_retries_only_confirmed_pre_admission_failures() {
    let terminal_id = Uuid::new_v4();
    let input_id = Uuid::new_v4();
    let capability = WriterCapability::generate();
    let host = FakeHost::start(vec![
        Reply::retryable_error(error_code::WORKER_UNREACHABLE),
        Reply::value(&host_status()),
        Reply::value(&running_session(terminal_id)),
        Reply::retryable_error(error_code::WORKER_UNREACHABLE),
        Reply::value(&host_status()),
        Reply::value(&running_session(terminal_id)),
        Reply::InputDelivered,
    ])
    .await;
    let target = RuntimeRef::new(terminal_id, test_runtime_epoch());
    let mut channel = ControlChannel::connect(RelayRuntime::existing(host.client()), target)
        .await
        .unwrap();

    let delivered = channel
        .input(GENERATION, &capability, input_id, INPUT)
        .await
        .unwrap();
    assert_eq!(delivered.input_id, input_id);
    let inputs = host
        .requests()
        .into_iter()
        .filter(|request| request.method == method::TERMINAL_INPUT)
        .map(|request| request.decode_payload::<InputRequest>().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(inputs.len(), INPUT_ATTEMPTS);
    assert!(inputs.iter().all(|input| input.input_id == input_id));
    assert!(inputs.iter().all(|input| input.data == INPUT));
    assert!(inputs.iter().all(|input| input.generation == GENERATION));
    assert!(
        inputs
            .iter()
            .all(|input| input.writer_capability == capability)
    );
}
