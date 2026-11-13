use super::query_constants::*;
use yolo_hub_identity::{HubId, TerminalAddress, TerminalId};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{
    CreateShellFields, CreateShellRequest, CreateShellResult, CreationQueryOutcome,
    CreationQueryRequest, CreationQueryResult, TERMINAL_PROTOCOL_SCHEMA_VERSION, SecretCapability,
    TerminalSize,
};

pub fn query() -> Command {
    Command::QueryCreation {
        request: CreationQueryRequest::new(
            CreateShellRequest::new(CreateShellFields {
                schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
                request_id: uuid::Uuid::new_v4(),
                creation_token: uuid::Uuid::new_v4(),
                name: None,
                cwd: QUERY_DIRECTORY.into(),
                size: TerminalSize::default(),
            })
            .unwrap(),
        ),
    }
}

pub fn ready(command: &Command, origin: HubId) -> Reply {
    let Command::QueryCreation { request } = command else {
        panic!()
    };
    Reply::CreationQuery {
        query: CreationQueryResult::new(
            request.creation().fields().creation_token,
            CreationQueryOutcome::Ready {
                creation: CreateShellResult {
                    address: TerminalAddress {
                        hub_id: origin,
                        terminal_id: TerminalId::new(),
                    },
                    creation_capability: SecretCapability::generate(),
                },
            },
        )
        .unwrap(),
    }
}
