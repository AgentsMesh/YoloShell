use yolo_hub_terminal_protocol::{
    CreateShellFields, CreateShellRequest, TERMINAL_PROTOCOL_SCHEMA_VERSION, TerminalSize,
};
use yolo_runtime::model::CreateTerminal;

const QUERY_NAME: &str = "original-query-name";
const QUERY_CWD: &str = "~/original-query-directory";
const QUERY_COLUMNS: u16 = 91;
const QUERY_ROWS: u16 = 26;
pub const UNSUPPORTED_QUERY_MESSAGE: &str = "creation query is unavailable on this Host";
pub const NON_RANDOM_QUERY_CAPABILITY: uuid::Uuid = uuid::Uuid::from_u128(1);

pub fn fields() -> CreateShellFields {
    CreateShellFields {
        schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
        request_id: uuid::Uuid::new_v4(),
        creation_token: uuid::Uuid::new_v4(),
        name: Some(QUERY_NAME.to_owned()),
        cwd: QUERY_CWD.to_owned(),
        size: TerminalSize::new(QUERY_COLUMNS, QUERY_ROWS).unwrap(),
    }
}

pub fn request() -> CreateShellRequest {
    CreateShellRequest::new(fields()).unwrap()
}

pub fn assert_host_request(actual: &CreateTerminal, original: &CreateShellRequest) {
    assert_eq!(actual.creation_token, original.fields().creation_token);
    assert_eq!(actual.name, original.fields().name);
    assert_eq!(actual.cwd.as_os_str(), QUERY_CWD);
    assert_eq!(actual.cols, Some(QUERY_COLUMNS));
    assert_eq!(actual.rows, Some(QUERY_ROWS));
}
