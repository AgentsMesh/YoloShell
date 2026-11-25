use super::fixtures;
use crate::{events, network::Update};
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::{ControlError, ControlEvent, ControlRequest};
use yolo_hub_identity::RuntimeEpoch;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{
    PublicError, TerminalAction, TerminalCommand, TerminalSelection, TerminalSummary,
};

const CURRENT_STATUS: &str = "Current terminal feedback";

#[test]
fn late_recovery_error_does_not_prompt_recreating_an_already_replaced_runtime() {
    for absent in [false, true] {
        let original = fixtures::session();
        let target = original.fields().runtime.unwrap().target;
        let mut fields = original.fields().clone();
        fields.runtime.as_mut().unwrap().target.runtime_epoch = RuntimeEpoch::new();
        let mut state = crate::client_capability_support::state();
        state.control_generation = Some(Uuid::new_v4());
        state.replace_terminals(vec![TerminalSummary::new(fields).unwrap()]);
        state.status = CURRENT_STATUS.into();
        let request = ControlRequest {
            generation: state.control_generation,
            validity: None,
            destination: target.address.hub_id,
            command: Command::Terminal {
                command: TerminalCommand {
                    request_id: Uuid::new_v4(),
                    selection: TerminalSelection {
                        address: target.address,
                        expected_runtime: (!absent).then_some(target.runtime_epoch),
                    },
                    action: TerminalAction::Resume {},
                },
            },
        };
        let (jobs, mut incoming) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
        events::apply(
            &mut state,
            &jobs,
            Update::Control(ControlEvent::Finished {
                request,
                result: Err(ControlError::Client(ClientError::Remote(
                    PublicError::RecreateRequired,
                ))),
                connected: true,
            }),
        );
        assert_eq!(state.status, CURRENT_STATUS);
        assert!(incoming.try_recv().is_err());
    }
}
