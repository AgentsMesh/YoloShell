pub const STATE_SCHEMA: u16 = 1;
pub const INITIAL_SEQUENCE: u64 = 0;
pub const REVISION_STEP: u64 = 1;
pub const INVALID_STATE: &str = "invalid connector registry state";
pub const INVALID_SNAPSHOT: &str = "connector snapshot disagrees with its enrollment";
pub const INVALID_PUBLICATION: &str = "connector publication disagrees with retained state";
