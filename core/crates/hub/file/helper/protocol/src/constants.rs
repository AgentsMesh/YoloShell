use std::time::Duration;

pub const FILE_HELPER_SCHEMA: u16 = 2;
pub const MAX_PROTECTED_PATHS: usize = 16;
pub const MAX_HELPER_LIFETIME: Duration = Duration::from_secs(15);
pub const MAX_OPERATION_TIME: Duration = Duration::from_secs(10);
pub const HELPER_EXIT_SUCCESS: u8 = 0;
pub const HELPER_EXIT_FAILURE: u8 = 1;
pub const HELPER_EXIT_EXPIRED: i32 = 70;

pub(crate) const ROOT_PREFIX: char = '/';
