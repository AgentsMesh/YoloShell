use super::*;
use crate::model::OutputChunk;
use crate::protocol::{EventEnvelope, event, method};
use crate::relay::session_test_support::{
    exit_event_at, host_status, running_session, test_runtime_epoch,
};
use crate::relay::test_support::{FakeHost, Reply};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

pub(super) const CAPTURE_BYTES: usize = 128;
pub(super) const COMPLETION_TIMEOUT: Duration = Duration::from_secs(3);
pub(super) const INITIAL_REQUEST_COUNT: usize = 1;
pub(super) const RECOVERED_REQUEST_COUNT: usize = 2;
const TAIL_SEQUENCE: u64 = crate::relay::INITIAL_OUTPUT_SEQUENCE + 1;

pub(super) fn ending_prefix(terminal_id: Uuid, attachment: Reply) -> Vec<Reply> {
    vec![
        Reply::value(&host_status()),
        Reply::value(&running_session(terminal_id)),
        attachment,
        Reply::error(crate::lease::constants::code::SESSION_NOT_RUNNING),
    ]
}

pub(super) fn output_event(terminal_id: Uuid, data: &[u8]) -> EventEnvelope {
    let chunk = OutputChunk {
        runtime_epoch: test_runtime_epoch(),
        sequence: TAIL_SEQUENCE,
        data: data.to_vec(),
    };
    EventEnvelope::new(
        event::TERMINAL_OUTPUT,
        Some(terminal_id.to_string()),
        Some(test_runtime_epoch().to_string()),
        &chunk,
    )
    .unwrap()
}

pub(super) fn tail_exit_event(terminal_id: Uuid) -> EventEnvelope {
    exit_event_at(terminal_id, TAIL_SEQUENCE)
}

pub(super) async fn await_gate(receiver: oneshot::Receiver<()>) {
    tokio::time::timeout(COMPLETION_TIMEOUT, receiver)
        .await
        .expect("attachment gate timed out")
        .expect("attachment gate closed");
}

pub(super) async fn await_session_ending(
    statuses: &mut mpsc::UnboundedReceiver<RelayStatus>,
) -> Vec<RelayStatus> {
    tokio::time::timeout(COMPLETION_TIMEOUT, async {
        let mut observed = Vec::new();
        loop {
            let status = statuses.recv().await.expect("relay status channel closed");
            let ending = status == RelayStatus::SessionEnding;
            observed.push(status);
            if ending {
                return observed;
            }
        }
    })
    .await
    .expect("session ending status timed out")
}

pub(super) async fn await_relay(
    relay: &mut tokio::task::JoinHandle<Result<RelayExit, RelayError>>,
) -> Result<RelayExit, RelayError> {
    match tokio::time::timeout(COMPLETION_TIMEOUT, &mut *relay).await {
        Ok(result) => result.expect("relay task panicked"),
        Err(_) => {
            relay.abort();
            panic!("relay completion timed out");
        }
    }
}

pub(super) fn assert_recovery_methods(
    host: &FakeHost,
    expected_attach: usize,
    expected_status: usize,
) {
    let methods = host.methods();
    assert_eq!(
        method_count(&methods, method::ATTACH_SESSION),
        expected_attach
    );
    assert_eq!(method_count(&methods, method::HOST_STATUS), expected_status);
    assert!(!methods.contains(&method::RESUME_SESSION_EXACT.into()));
    assert!(!methods.contains(&method::RELEASE_WRITER.into()));
}

fn method_count(methods: &[String], expected: &str) -> usize {
    methods
        .iter()
        .filter(|method| method.as_str() == expected)
        .count()
}
