pub const FILE_PROTOCOL_SCHEMA_VERSION: u16 = 1;
pub const MAX_FILE_READ_BYTES: u32 = 128 * 1024;
pub const MAX_FILE_OFFSET: u64 = i64::MAX as u64;
pub const MAX_FILE_PATH_BYTES: usize = 4096;
pub const MAX_FILE_COMPONENT_BYTES: usize = 255;
pub const MAX_FILE_PATH_DEPTH: usize = 64;
pub const MAX_FILE_ROOT_NAME_BYTES: usize = 128;
pub const MAX_FILE_ROOTS: usize = 64;
pub const MAX_FILE_ROOT_PAGE: usize = 16;

pub(crate) const PATH_SEPARATOR: char = '/';
pub(crate) const CURRENT_COMPONENT: &str = ".";
pub(crate) const PARENT_COMPONENT: &str = "..";
pub(crate) const REDACTED_FILE_CONTENT: &str = "<file contents redacted>";
