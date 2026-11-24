use uuid::Uuid;
use yolo_hub_client_channels::ControlRequest;
use yolo_hub_identity::{HubId, TerminalAddress, TerminalId};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{
    CreateShellFields, CreateShellRequest, CreateShellResult, CreationQueryOutcome,
    CreationQueryResult, TERMINAL_PROTOCOL_SCHEMA_VERSION, SecretCapability, TerminalSize,
};
use yolo_tui_creation_journal::CreationIntent;

pub const DIRECTORY: &str = "/private/recovery-fixture";
pub const OTHER_DIRECTORY: &str = "/private/changed-recovery-fixture";
pub const NAME: &str = "recovered shell";
pub const FIRST: usize = 0;
pub const ONE: usize = 1;

pub fn intent() -> CreationIntent {
    CreationIntent::new(
        HubId::new(),
        CreateShellRequest::new(CreateShellFields {
            schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
            request_id: Uuid::new_v4(),
            creation_token: Uuid::new_v4(),
            name: Some(NAME.into()),
            cwd: DIRECTORY.into(),
            size: TerminalSize::default(),
        })
        .unwrap(),
    )
    .unwrap()
}

pub fn changed(
    original: &CreationIntent,
    change: impl FnOnce(&mut CreateShellFields),
) -> CreationIntent {
    let mut fields = original.request.fields().clone();
    change(&mut fields);
    CreationIntent::new(original.origin, CreateShellRequest::new(fields).unwrap()).unwrap()
}

pub fn creation(intent: &CreationIntent) -> CreateShellResult {
    CreateShellResult {
        address: TerminalAddress {
            hub_id: intent.origin,
            terminal_id: TerminalId::new(),
        },
        creation_capability: SecretCapability::generate(),
    }
}

pub fn reply(intent: &CreationIntent, outcome: CreationQueryOutcome) -> Reply {
    Reply::CreationQuery {
        query: CreationQueryResult::new(intent.request.fields().creation_token, outcome).unwrap(),
    }
}

pub fn ready(intent: &CreationIntent) -> Reply {
    reply(
        intent,
        CreationQueryOutcome::Ready {
            creation: creation(intent),
        },
    )
}

pub fn requested(request: &ControlRequest) -> CreationIntent {
    let Command::QueryCreation { request: query } = &request.command else {
        panic!()
    };
    CreationIntent::new(request.destination, query.creation().clone()).unwrap()
}
