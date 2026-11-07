use crate::fixtures::{self, constants::*};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::*;

pub const REDACTED: &str = "[REDACTED]";

pub fn command() -> Command {
    Command::QueryCreation {
        request: CreationQueryRequest::new(fixtures::create()),
    }
}
pub fn outcomes(origin: u128) -> Vec<CreationQueryOutcome> {
    let mut creation = fixtures::creation();
    creation.address.hub_id = fixtures::hub(origin);
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
pub fn reply(outcome: CreationQueryOutcome, token: uuid::Uuid) -> Reply {
    Reply::CreationQuery {
        query: CreationQueryResult::new(token, outcome).unwrap(),
    }
}
pub fn token() -> uuid::Uuid {
    fixtures::id(CREATION_ID)
}
