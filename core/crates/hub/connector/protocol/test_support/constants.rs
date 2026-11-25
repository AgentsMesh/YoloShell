pub const PROVIDER_KIND: &str = "example.connector";
pub const PROVIDER_VERSION: &str = "1.0.0";
pub const PROVIDER_REFERENCE: &str = "provider-native-thread";
pub const DIFFERENT_VERSION: &str = "1.0.1";
pub const DIRECTORY: &str = "/remote/workspace";
pub const OBSERVATION_SEQUENCE: u64 = 7;
pub const REVISION_INCREMENT: u64 = 1;
pub const INVALID_PROTOCOL_VERSIONS: [u16; 2] = [0, u16::MAX];
pub const UNKNOWN_FIELD: &str = "unchecked_authority";
pub const UNKNOWN_TEXT: &str = "unexpected";
pub const UUID_TEXT: &str = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";
pub const NON_RANDOM_UUID: &str = "aaaaaaaa-bbbb-1ccc-8ddd-eeeeeeeeeeee";
pub const INVALID_VARIANT_UUID: &str = "aaaaaaaa-bbbb-4ccc-0ddd-eeeeeeeeeeee";
pub const FORBIDDEN_STATE_FIELDS: &[&str] = &[
    "capabilities",
    "interaction",
    "source",
    "recovery",
    "pid",
    "process_pid",
    "process_start_time",
];
