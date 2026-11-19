use crate::fixtures;
use yolo_hub_terminal_protocol::*;

pub const OUTCOME_FIELD: &str = "outcome";
pub const STATUS_FIELD: &str = field::STATUS;
pub const CREATION_FIELD: &str = "creation";
pub const CAPABILITY_FIELD: &str = field::CREATION_CAPABILITY;
pub const READY_STATUS: &str = "ready";
pub const QUERY_REDACTED: &str = "[REDACTED]";

pub fn request() -> CreationQueryRequest {
    CreationQueryRequest::new(CreateShellRequest::new(fixtures::create_fields()).unwrap())
}

pub fn creation() -> CreateShellResult {
    CreateShellResult {
        address: fixtures::address(),
        creation_capability: fixtures::grant_fields().capability,
    }
}

pub fn outcomes() -> Vec<CreationQueryOutcome> {
    let creation = creation();
    vec![
        CreationQueryOutcome::Missing {},
        CreationQueryOutcome::Pending {},
        CreationQueryOutcome::Unknown {},
        CreationQueryOutcome::Ready { creation },
        CreationQueryOutcome::Finalized { creation },
        CreationQueryOutcome::Survivor { creation },
        CreationQueryOutcome::Archived {
            address: creation.address,
        },
    ]
}

pub fn result(outcome: CreationQueryOutcome) -> CreationQueryResult {
    CreationQueryResult::new(request().creation().fields().creation_token, outcome).unwrap()
}
