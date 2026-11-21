pub const BOUNDARY_STEP: u64 = 1;
pub const ABOVE_RUNTIME_LIMIT: u64 = yolo_build_image::MAX_EXECUTABLE_IMAGE_BYTES + BOUNDARY_STEP;
pub const KIND_FIELD: &str = "kind";
pub const RUNTIME_KIND: &str = "runtime";
pub const CHANGED_LAST_BYTE: u8 = 0xa5;
pub const INVALID_EXECUTABLE: &str = "Codex admission executable is invalid";
