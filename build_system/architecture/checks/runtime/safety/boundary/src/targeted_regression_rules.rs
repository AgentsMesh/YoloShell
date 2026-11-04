use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_source_policy::source_policy::is_rust_test_source;
use yolo_architecture_syntax::targeted_tokens::tokenize;

pub const FRAME_WIDTH_MESSAGE: &str =
    "targeted regression: protocol frame widths must use FRAME_LENGTH_BYTES";
pub const PERMISSION_MASK_MESSAGE: &str =
    "targeted regression: production permission checks must use a named mode mask";
pub const HASHED_HOST_MESSAGE: &str =
    "targeted regression: hashed known_hosts parsing must use format-aware prefix removal";
pub(crate) const HASHED_HOST_MATCHER_PATH: &str =
    "core/crates/transport/remote/src/host_key_match.rs";
const REMOTE_SERVER_PROTOCOL_PATH: &str =
    "core/crates/transport/remote/test_support/russh_backend/server_protocol.rs";
pub const PIPE_DESCRIPTOR_MESSAGE: &str =
    "targeted regression: pipe descriptor counts and indexes must use named constants";
pub const HEX_CAPACITY_MESSAGE: &str =
    "targeted regression: hex encoding capacities must use a named encoded-length constant";

pub fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    let tokens = tokenize(&file.text);
    if protocol_source(&file.relative) {
        crate::targeted_frame_rules::inspect(file, &tokens, violations);
    }
    if production_rust(&file.relative) {
        crate::targeted_rust_rules::inspect_permission_masks(file, &tokens, violations);
        crate::targeted_rust_rules::inspect_hex_capacities(file, &tokens, violations);
    }
    if hashed_host_matcher(&file.relative) {
        crate::targeted_rust_rules::inspect_hashed_host_fields(file, &tokens, violations);
    }
    if swift_pipe(&file.relative) {
        crate::targeted_swift_rules::inspect_pipe_descriptors(file, &tokens, violations);
    }
}

fn protocol_source(relative: &str) -> bool {
    relative == "core/src/protocol.rs"
        || relative.starts_with("core/src/protocol/")
        || relative.starts_with("core/crates/wire/src/")
        || relative == "integration_tests/runtime/protocol.rs"
        || relative == REMOTE_SERVER_PROTOCOL_PATH
}

fn production_rust(relative: &str) -> bool {
    (relative.starts_with("core/src/") || relative.starts_with("core/crates/"))
        && !is_rust_test_source(relative)
}

fn hashed_host_matcher(relative: &str) -> bool {
    relative == HASHED_HOST_MATCHER_PATH
}

fn swift_pipe(relative: &str) -> bool {
    relative.ends_with("/HelperProcessPipe.swift")
        || relative.ends_with("/HelperPrivateInputPipe.swift")
}
