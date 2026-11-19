pub const RUST_SOURCE_SUFFIX: &str = ".rs";
pub const PRE_EXEC_METHOD: &str = "pre_exec";
pub const SETRLIMIT_FUNCTION: &str = "setrlimit";
pub const POLICY_MESSAGE: &str =
    "setrlimit must run before spawn, never inside a CommandExt::pre_exec callback";

pub const MEMBER_ACCESS: &str = ".";
pub const PATH_SEPARATOR: &str = "::";
pub const CALL_OPEN: &str = "(";
pub const CALL_CLOSE: &str = ")";
pub const BLOCK_OPEN: &str = "{";
pub const BLOCK_CLOSE: &str = "}";
pub const ARRAY_OPEN: &str = "[";
pub const ARRAY_CLOSE: &str = "]";
pub const CLOSURE_BAR: &str = "|";
pub const ARGUMENT_SEPARATOR: &str = ",";
pub const ASSIGNMENT: &str = "=";
pub const ITEM_END: &str = ";";
pub const MOVE_KEYWORD: &str = "move";
pub const ASYNC_KEYWORD: &str = "async";
pub const RETURN_KEYWORD: &str = "return";
pub const NEXT_TOKEN_OFFSET: usize = 1;
