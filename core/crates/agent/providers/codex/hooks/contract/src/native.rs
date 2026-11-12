pub const NATIVE_SCHEMA_VERSION: u16 = 2;
pub use yolo_agent_runtime::{
    NATIVE_CALLBACK_SHELLS, NATIVE_HOOK_EXECUTABLE_ENV, NATIVE_HOOK_HOME_ENV,
};
pub const NATIVE_DATA_HOME_ENV: &str = "CODEX_HOME";
pub const NATIVE_USER_HOME_ENV: &str = "HOME";
pub const NATIVE_DATA_DIRECTORY_NAME: &str = ".codex";
pub const NATIVE_ARTIFACTS_DIRECTORY_NAME: &str = "codex-native-hooks";
pub const NATIVE_REQUEST_SUFFIX: &str = ".request.json";
pub const NATIVE_MAX_DATA_HOME_CANDIDATES: usize = 2;
pub const NATIVE_INVOCATION_DOMAIN: &str = "yolo.codex-native.invocation.v1";
