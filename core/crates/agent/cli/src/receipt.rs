use crate::constants::{EXIT_REJECTED, EXIT_UNKNOWN};
use crate::outcome::Outcome;
use anyhow::{Result, ensure};
use serde::Serialize;
use uuid::Uuid;
use yolo_hub_agent_protocol::{CommandOutcome, CommandReceipt};
use yolo_hub_identity::AgentTarget;

pub enum Execution {
    Receipt(CommandReceipt),
    Unknown,
}

#[derive(Serialize)]
struct ReceiptValue {
    request_id: Uuid,
    target: AgentTarget,
    outcome: CommandOutcome,
}

#[derive(Serialize)]
struct FailureValue {
    code: &'static str,
    message: String,
}

#[derive(Serialize)]
struct UnknownValue {
    error: FailureValue,
    request_id: Uuid,
    target: AgentTarget,
}

pub fn outcome(
    execution: Execution,
    request_id: Uuid,
    target: AgentTarget,
    json_mode: bool,
) -> Result<Outcome> {
    let Execution::Receipt(receipt) = execution else {
        return unknown_outcome(request_id, target, json_mode);
    };
    ensure!(
        receipt.request_id() == request_id && receipt.target() == target,
        crate::constants::UNEXPECTED_REPLY
    );
    let outcome = receipt.outcome();
    Ok(match outcome {
        CommandOutcome::Confirmed {} => Outcome::success(
            json_mode,
            receipt_value(request_id, target, outcome)?,
            format!("Confirmed {request_id}"),
        ),
        CommandOutcome::Delivered {} => Outcome::success(
            json_mode,
            receipt_value(request_id, target, outcome)?,
            format!("Delivered {request_id}"),
        ),
        CommandOutcome::Accepted {} => Outcome::success(
            json_mode,
            receipt_value(request_id, target, outcome)?,
            format!("Accepted {request_id}"),
        ),
        CommandOutcome::Failed { .. } => {
            let mut failure = Outcome::failure(
                EXIT_REJECTED,
                json_mode,
                format!("control rejected: {outcome:?} (request {request_id})"),
            );
            failure.value = receipt_value(request_id, target, outcome)?;
            failure
        }
        CommandOutcome::Unknown {} => return unknown_outcome(request_id, target, json_mode),
    })
}

fn receipt_value(
    request_id: Uuid,
    target: AgentTarget,
    outcome: CommandOutcome,
) -> Result<serde_json::Value> {
    Ok(serde_json::to_value(ReceiptValue {
        request_id,
        target,
        outcome,
    })?)
}

pub fn unknown_outcome(request_id: Uuid, target: AgentTarget, json_mode: bool) -> Result<Outcome> {
    let message =
        format!("control outcome is unknown for request {request_id}; do not retry automatically");
    let mut outcome = Outcome::failure(EXIT_UNKNOWN, json_mode, message.clone());
    outcome.value = serde_json::to_value(UnknownValue {
        error: FailureValue {
            code: "unknown",
            message,
        },
        request_id,
        target,
    })?;
    Ok(outcome)
}
