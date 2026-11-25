//! Workspace protocol constants.

pub const WORKSPACE_PROTOCOL_SCHEMA_VERSION: u16 = 1;
/// Serde tag carried by an addressable or membership change operation.
pub const CHANGE_TAG: &str = "change";
pub const MAX_WORKSPACE_NAME_BYTES: usize = 128;
pub const MAX_WORKSPACE_ROOT_BYTES: usize = 4_096;
pub const MAX_WORKSPACE_LABELS: usize = 32;
pub const MAX_WORKSPACE_LABEL_KEY_BYTES: usize = 64;
pub const MAX_WORKSPACE_LABEL_VALUE_BYTES: usize = 256;
pub const MAX_WORKSPACE_PAGE: usize = 128;
/// Maximum number of member ids in one bind/unbind change.
pub const MAX_WORKSPACE_MEMBER_CHANGES: usize = 128;
pub const FIRST_WORKSPACE_REVISION: u64 = 1;
