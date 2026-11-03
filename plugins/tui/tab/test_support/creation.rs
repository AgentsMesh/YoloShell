use super::*;
use yolo_hub_terminal_protocol::{
    CreateShellFields, CreateShellRequest, TERMINAL_PROTOCOL_SCHEMA_VERSION,
};
use yolo_tui_creation_journal::CreationIntent;

pub fn intent(hub: HubId) -> CreationIntent {
    let request = CreateShellRequest::new(CreateShellFields {
        schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
        request_id: uuid::Uuid::new_v4(),
        creation_token: uuid::Uuid::new_v4(),
        name: None,
        cwd: JOURNAL_CWD.into(),
        size: Default::default(),
    })
    .unwrap();
    CreationIntent::new(hub, request).unwrap()
}
