pub const RECORD_FILE: &str = "agent-projection.json";
pub const LOCK_FILE: &str = "agent-projection.lock";
pub const NAMESPACE: &str = "hub-agent-projection";
pub const SCHEMA_VERSION: u16 = 5;
/// Previous record schema accepted and migrated on load.
pub const PREVIOUS_SCHEMA_VERSION: u16 = 4;
/// The snapshot schema carried by every record written at record schema 4.
pub const RECORD_4_SNAPSHOT_SCHEMA: u16 = 2;
pub const MAX_AGENT_RECORDS: usize = 4_096;
pub const MAX_LOCK_BYTES: u64 = 4_096;
pub const MAX_PRIVATE_METADATA_BYTES: usize = 4_096;
pub const ALREADY_OPEN: &str = "agent projection is already owned by another writer";
pub const INVALID_RECORD: &str = "agent projection record is invalid";
pub const WRONG_ORIGIN: &str = "agent projection belongs to a different Hub";
pub const REPLACED_DIRECTORY: &str = "agent projection directory identity changed";
pub const INVALID_BINDING: &str = "Host agent binding has invalid identity metadata";
pub const STALE_BINDING: &str = "Host agent binding predates the accepted observation";
pub const RECORD_LIMIT: &str = "agent projection identity retention limit reached";
pub const CURSOR_EXHAUSTED: &str = "agent projection observation cursor exhausted";
pub const EXITED_PROCESS_REAPPEARED: &str =
    "confirmed exited agent process cannot become active again";
