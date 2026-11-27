pub const ROOT_SOURCE: &str = "core/crates/transport/remote/broker/src/mod.rs";
pub const SOURCE_PREFIX: &str = "core/crates/transport/remote/broker/src/";
pub const RUST_SUFFIX: &str = ".rs";
pub const MESSAGE: &str = "remote broker production code cannot create or escape a local process";
pub const STANDARD_LIBRARY_NAMESPACE: &str = "std";
pub const ASYNC_RUNTIME_NAMESPACE: &str = "tokio";
pub const PROCESS_COMMAND_NAMESPACES: &[&str] =
    &[STANDARD_LIBRARY_NAMESPACE, ASYNC_RUNTIME_NAMESPACE];
pub const PROCESS_ESCAPE_IDENTIFIERS: &[&str] = &[
    "CommandBuilder",
    "SYS_clone",
    "SYS_clone3",
    "SYS_execve",
    "SYS_execveat",
    "SYS_fork",
    "SYS_vfork",
    "clone3",
    "daemon",
    "dlsym",
    "execle",
    "execlp",
    "execl",
    "execve",
    "execveat",
    "execvp",
    "execvpe",
    "execv",
    "fexecve",
    "fork",
    "forkpty",
    "link_name",
    "popen",
    "posix_spawn",
    "posix_spawnp",
    "setpgid",
    "setsid",
    "spawn_command",
    "system",
    "syscall",
    "vfork",
];
pub const PROCESS_WRAPPER_CRATES: &[&str] = &[
    "async_process",
    "duct",
    "libloading",
    "portable_pty",
    "process_control",
    "subprocess",
];
pub const CLONE_IDENTIFIER: &str = "clone";
pub const COMMAND_IDENTIFIER: &str = "Command";
pub const CONSTRUCTOR_IDENTIFIER: &str = "new";
pub const FUNCTION_KEYWORD: &str = "fn";
pub const LIBC_NAMESPACE: &str = "libc";
pub const PROCESS_NAMESPACE: &str = "process";
pub const PATH_SEPARATOR: &str = "::";
pub const USE_KEYWORD: &str = "use";
pub const STATEMENT_END: &str = ";";
