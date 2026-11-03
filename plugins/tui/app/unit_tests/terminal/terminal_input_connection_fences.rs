use super::{query_support::Fixture, terminal_input_ordering::queued};
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::ControlError;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{InputRequest, PublicError};

const INPUT: &[u8] = b"original input";

#[test]
fn completed_and_unknown_input_receipts_cannot_manufacture_a_disconnect_even_without_validity() {
    for unknown in [false, true] {
        let mut fixture = Fixture::new();
        let mut stale = queued(&mut fixture, INPUT);
        fixture.finish(stale.clone(), Ok(Reply::Input { duplicate: false }), true);
        if unknown {
            let Command::Input { request } = stale.command else {
                panic!()
            };
            let mut fields = request.fields().clone();
            fields.request_id = uuid::Uuid::new_v4();
            stale.command = Command::Input {
                request: InputRequest::new(fields).unwrap(),
            };
        }
        stale.validity = None;
        let generation = fixture.state.control_generation;
        fixture.finish(
            stale,
            Err(ControlError::Client(ClientError::Poisoned)),
            false,
        );
        assert_eq!(fixture.state.control_generation, generation);
        assert_eq!(
            fixture.state.writers[&fixture.target.address].grant,
            fixture.grant
        );
        fixture.no_jobs();
    }
}

#[test]
fn matched_current_input_failure_reports_real_control_channel_loss() {
    let mut fixture = Fixture::new();
    let request = queued(&mut fixture, INPUT);
    fixture.finish(
        request,
        Err(ControlError::Client(ClientError::OutcomeUnknownResponse(
            PublicError::InvalidPayload,
        ))),
        false,
    );
    assert!(fixture.state.control_generation.is_none());
    assert!(fixture.state.writers.is_empty());
    assert!(!fixture.state.status.is_empty());
    fixture.no_jobs();
}
