//! Closed outcome vocabulary for the private relay wire format.
use serde::{Serialize};
use yolo_hub_agent_protocol::CommandReceipt;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NotSentReason {
    NoWriter,
    Expired,
    TargetChanged,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case", deny_unknown_fields)]
pub enum Outcome {
    Receipt {
        receipt: CommandReceipt,
    },
    NotSent {
        reason: NotSentReason,
    },
    Unknown {},
}
