pub const FAILURE_PROBE_ENV: &str = "YOLO_COVERAGE_FAILURE_PROBE";
pub const ENABLED: &str = "1";
pub const CORRUPT_PROFILE: &str = "deliberately-invalid.profraw";
pub const CORRUPT_PROFILE_CONTENT: &[u8] = b"not an LLVM profile";
pub const COVERAGE_DIRECTORY: &str = "COVERAGE_DIR";
pub const PREEXISTING_MARKER_PROBE: &str = "preserve-existing-marker";
pub const PREMATURE_EXIT_ENV: &str = "TEST_PREMATURE_EXIT_FILE";
pub const PREEXISTING_FAILURE: &[u8] = b"prior test phase did not complete";
