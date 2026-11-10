use crate::constants::{EXIT_REJECTED, EXIT_SUCCESS, EXIT_UNKNOWN};
use crate::receipt::{Execution, outcome, unknown_outcome};
use uuid::Uuid;
use yolo_hub_agent_protocol::{CommandOutcome, CommandReceipt, FailureCode, REQUEST_ID_FIELD};
use yolo_hub_identity::{AgentAddress, AgentId, AgentTarget, ExecutionId, HubId};

fn target() -> AgentTarget {
    AgentTarget {
        address: AgentAddress {
            hub_id: HubId::new(),
            agent_id: AgentId::new(),
        },
        execution_id: ExecutionId::new(),
    }
}

fn receipt(outcome: CommandOutcome) -> (Uuid, AgentTarget, CommandReceipt) {
    let request_id = Uuid::new_v4();
    let target = target();
    let mut receipt = CommandReceipt::accepted(request_id, target).unwrap();
    if !matches!(outcome, CommandOutcome::Accepted {}) {
        receipt.advance(outcome).unwrap();
    }
    (request_id, target, receipt)
}

#[test]
fn delivered_and_accepted_receipts_succeed() {
    for (label, value) in [
        ("delivered", CommandOutcome::Delivered {}),
        ("accepted", CommandOutcome::Accepted {}),
    ] {
        let (request_id, target, receipt) = receipt(value);
        let outcome = outcome(Execution::Receipt(receipt), request_id, target, true).unwrap();
        assert_eq!(outcome.code, EXIT_SUCCESS, "{label}");
        assert_eq!(outcome.value["outcome"]["state"], label);
    }
}

#[test]
fn a_failed_receipt_is_rejected() {
    let (request_id, target, receipt) = receipt(CommandOutcome::Failed {
        code: FailureCode::PreconditionsChanged,
    });
    let outcome = outcome(Execution::Receipt(receipt), request_id, target, true).unwrap();
    assert_eq!(outcome.code, EXIT_REJECTED);
    assert_eq!(outcome.value["outcome"]["state"], "failed");
}

#[test]
fn unknown_execution_and_receipt_exit_unknown() {
    let target = target();
    let request_id = Uuid::new_v4();
    let outcome = crate::receipt::outcome(Execution::Unknown, request_id, target, true).unwrap();
    assert_eq!(outcome.code, EXIT_UNKNOWN);
    assert_eq!(outcome.value[REQUEST_ID_FIELD], request_id.to_string());

    let unknown = unknown_outcome(request_id, target, false).unwrap();
    assert_eq!(unknown.code, EXIT_UNKNOWN);
    assert!(unknown.text.contains("do not retry automatically"));
}
