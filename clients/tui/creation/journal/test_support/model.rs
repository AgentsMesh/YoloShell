pub(crate) use crate::constants::FIELD_INTENTS as INTENTS_FIELD;
pub const UNSUPPORTED_SCHEMA_VERSION: u16 = 2;
use crate::{CreationIntent, PendingFinalization};
use uuid::Uuid;
use yolo_hub_identity::{HubId, TerminalAddress, TerminalId};
pub use yolo_hub_terminal_protocol::field::{
    CREATION_TOKEN as TOKEN_FIELD, SCHEMA_VERSION as SCHEMA_FIELD,
};
use yolo_hub_terminal_protocol::{
    CreateShellFields, CreateShellRequest, CreateShellResult, TERMINAL_PROTOCOL_SCHEMA_VERSION,
    SecretCapability, TerminalSize,
};
pub use yolo_wire::field::CREATION_CAPABILITY as CAPABILITY_FIELD;

pub const REQUEST_DIRECTORY: &str = "/workspace";
pub const OTHER_REQUEST_DIRECTORY: &str = "/other-workspace";
pub const SESSION_NAME: &str = "confirmed shell";
pub const PENDING_FIELD: &str = "pending";
pub const UNKNOWN_FIELD: &str = "unknown";
pub const REVISION_FIELD: &str = "revision";
pub const INVALID_JSON: &[u8] = b"{";
pub const FIRST_INDEX: usize = 0;
pub const SCHEMA_STEP: u16 = 1;
pub fn directory_identity() -> yolo_paths::identity::DurableObjectIdentity {
    let root = tempfile::tempdir().unwrap();
    yolo_paths::directory::PrivateDirectory::ensure(root.path())
        .unwrap()
        .durable_identity()
        .unwrap()
}

pub fn pending(origin: HubId) -> PendingFinalization {
    PendingFinalization::new(
        origin,
        CreateShellRequest::new(CreateShellFields {
            schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
            request_id: Uuid::new_v4(),
            creation_token: Uuid::new_v4(),
            name: Some(SESSION_NAME.into()),
            cwd: REQUEST_DIRECTORY.into(),
            size: TerminalSize::default(),
        })
        .unwrap(),
        CreateShellResult {
            address: TerminalAddress {
                hub_id: origin,
                terminal_id: TerminalId::new(),
            },
            creation_capability: SecretCapability::generate(),
        },
    )
    .unwrap()
}

pub fn change_request(
    record: &mut PendingFinalization,
    change: impl FnOnce(&mut CreateShellFields),
) {
    let mut fields = record.request.fields().clone();
    change(&mut fields);
    record.request = CreateShellRequest::new(fields).unwrap();
}

pub fn intent(record: &PendingFinalization) -> CreationIntent {
    CreationIntent::new(record.origin, record.request.clone()).unwrap()
}

pub fn conflicting_confirmations(original: &PendingFinalization) -> Vec<PendingFinalization> {
    let mut values = Vec::new();
    let mut remote = original.clone();
    remote.origin = HubId::new();
    remote.creation.address.hub_id = remote.origin;
    values.push(remote);
    for change in [
        (|fields: &mut CreateShellFields| fields.request_id = Uuid::new_v4())
            as fn(&mut CreateShellFields),
        |fields| fields.creation_token = Uuid::new_v4(),
        |fields| fields.cwd = OTHER_REQUEST_DIRECTORY.into(),
        |fields| fields.name = None,
        |fields| {
            fields.size = TerminalSize::new(
                yolo_hub_terminal_protocol::MIN_TERMINAL_COLUMNS,
                fields.size.rows(),
            )
            .unwrap()
        },
    ] {
        let mut changed = original.clone();
        change_request(&mut changed, change);
        values.push(changed);
    }
    values
}
