pub const BINDING_SCHEMA_VERSION: u32 = 5;
pub const JSONL_EXTENSION: &str = "jsonl";
pub const HOME_ENV: &str = "HOME";
pub const YOLO_TERMINAL_ID_ENV: &str = "YOLO_TERMINAL_ID";
pub const YOLO_RUNTIME_EPOCH_ENV: &str = "YOLO_RUNTIME_EPOCH";
pub const YOLO_HUB_ID_ENV: &str = "YOLO_HUB_ID";
pub const YOLO_WORKSPACE_ID_ENV: &str = "YOLO_WORKSPACE_ID";
pub const YOLO_RUN_ID_ENV: &str = "YOLO_RUN_ID";
pub const ENVIRONMENT_ASSIGNMENT: char = '=';
pub const ENVIRONMENT_NUL: char = '\0';
pub const RESERVED_ENVIRONMENT_PREFIX: &str = "YOLO_";
pub const MAX_PROVIDER_ENVIRONMENT_VARIABLES: usize = 64;
pub const MAX_PROVIDER_ENVIRONMENT_BYTES: usize = 64 * 1_024;
pub const MIN_PROCESS_PID: u32 = 1;
pub const NODE_EXECUTABLES: &[&str] = &["node", "node.exe", "nodejs"];

pub mod message {
    pub const PROVIDER_MANIFEST_BOUNDARY: &str =
        "provider manifest has invalid role or execution boundary";
    pub const PROVIDER_MANIFEST_DEFINITION: &str = "provider manifest has no provider definition";
    pub const PROVIDER_MANIFEST_IDENTITY: &str =
        "provider manifest id does not match its provider definition";
    pub const PROVIDER_DEFINITION_ADAPTER: &str = "provider definition has no built-in adapter";
    pub const AGENT_SESSION_ID: &str =
        "agent binding session id must be non-empty without surrounding whitespace";
    pub const CWD: &str = "agent binding working directory is invalid";
    pub const DATA_HOME: &str = "agent binding data home does not match its agent kind";
    pub const DECODE: &str = "decode agent binding";
    pub const EVIDENCE_PATH: &str = "agent binding evidence path is outside its agent data home";
    pub const EVIDENCE_READ: &str = "read agent binding evidence";
    pub const ENVIRONMENT_INVALID: &str = "provider environment contains an invalid variable";
    pub const ENVIRONMENT_LIMIT: &str = "provider environment exceeds its budget";
    pub const REGISTRY_KIND_EMPTY: &str = "agent adapter kind cannot be empty";
    pub const REGISTRY_KIND_DUPLICATE: &str = "agent adapter kinds must be unique";
    pub const BUILTIN_REGISTRY: &str = "built-in agent registry must be valid";
    pub const REGISTERED_ADAPTER: &str = "detector kind must have a registered adapter";
    pub const SINGLE_CANDIDATE: &str = "detector candidate count must be one";
    pub const SINGLE_EVIDENCE: &str = "process evidence candidate count must be one";
    pub const EXECUTABLE: &str = "agent binding executable does not match its agent kind";
    pub const KIND: &str = "agent binding kind has no registered adapter";
    pub const GENERATION_AUTHORITY: &str =
        "agent binding does not match its durable generation authority";
    pub const PERSIST: &str = "persist agent binding";
    pub const PROCESS_PID: &str = "agent binding process pid is invalid";
    pub const PROCESS_START_TIME: &str = "agent binding process start time is invalid";
    pub const RECOVERY_AVAILABILITY: &str = "agent recovery availability violates provider policy";
    pub const READ: &str = "read agent binding";
    pub const REMOVE: &str = "remove agent binding";
    pub const SCHEMA: &str = "unsupported agent binding schema version";
    pub const STALE_EVIDENCE: &str = "agent binding evidence no longer matches the saved agent";
    pub const TIMESTAMPS: &str = "agent binding timestamps are invalid";
    pub const YOLO_SESSION: &str = "agent binding belongs to another Yolo session";
}
