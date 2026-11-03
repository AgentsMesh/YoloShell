pub const BOUNDARY_STEP: u64 = 1;
pub const RUNTIME_SIZED_EXECUTABLE: u64 =
    crate::codex_hooks::MAX_PROVIDER_EXECUTABLE_BYTES + BOUNDARY_STEP;
pub const KIND_FIELD: &str = "kind";
pub const RUNTIME_KIND: &str = "runtime";
pub const CHANGED_LAST_BYTE: u8 = 0xa5;
pub const INVALID_EXECUTABLE: &str = "Codex admission executable is invalid";
