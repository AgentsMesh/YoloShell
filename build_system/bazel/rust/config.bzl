
load(
    "//build_system/architecture:topology.bzl",
    "CAPNP_VALUE",
    "YOLO_LAUNCH_CONTRACT",
)
FAKE_AGENT_DEPS = [
    "//core:yolo_runtime",
    "@crates//:libc",
    "@crates//:serde",
    "@crates//:serde_json",
    "@crates//:uuid",
]

CLAUDE_SCHEMA_FIXTURES = [
    "//core/crates/agent/providers/claude:schema_fixture",
]

FAKE_CLAUDE_CONTRACT_SRCS = [
    "fake_agent/claude_scenario.rs",
    "fake_agent/claude_wire.rs",
    "fake_agent/claude_wire_path.rs",
    "fake_agent/claude_wire_validation.rs",
    "fake_claude/constants.rs",
    "fake_claude_authority_contract.rs",
    "fake_claude_authority_tests.rs",
    "fake_claude_contract.rs",
    "fake_claude_ordering_tests.rs",
    "fake_claude_process.rs",
    "fake_claude_process_custody.rs",
    "fake_claude_process_custody_tests.rs",
    "fake_claude_process_stream.rs",
    "fake_claude_process_tests.rs",
    "fake_claude_reference_endpoint.rs",
    "fake_claude_reference_store.rs",
    "fake_claude_reference_transition.rs",
    "fake_claude_reference_types.rs",
    "fake_claude_scenarios.rs",
    "fake_claude_secure_transcript.rs",
    "fake_claude_security_tests.rs",
]

RUNTIME_DEPS = [
    YOLO_LAUNCH_CONTRACT,
    "@crates//:anyhow",
    "@crates//:base64",
    "@crates//:crc32fast",
    "@crates//:futures",
    "@crates//:getrandom",
    "@crates//:hmac",
    "@crates//:libc",
    "@crates//:portable-pty",
    "@crates//:rmp-serde",
    CAPNP_VALUE,
    "@crates//:rusqlite",
    "@crates//:russh",
    "@crates//:russh-sftp",
    "@crates//:serde",
    "@crates//:serde_bytes",
    "@crates//:serde_json",
    "@crates//:sha1",
    "@crates//:sha2",
    "@crates//:thiserror",
    "@crates//:tokio",
    "@crates//:tracing",
    "@crates//:uuid",
    "@crates//:vt100",
    "@crates//:zeroize",
]

TEST_ENVIRONMENT = [
    "HOME",
    "PATH",
]

RUST_UNIT_TEST_THREAD_COUNT = "1"

RUNTIME_ARCHITECTURE_DATA = [
    "BUILD.bazel",
    "//integration_tests/runtime/catalog:architecture_check_data",
    "//integration_tests/runtime/durable_identity:architecture_check_data",
    "//integration_tests/runtime/failed_rearm:architecture_check_data",
    "//integration_tests/runtime/journal:architecture_check_data",
    "//integration_tests/runtime/protocol:architecture_check_data",
    "//integration_tests/runtime/release_audit:architecture_check_data",
    "//integration_tests/runtime/socket_root:architecture_check_data",
    "//integration_tests/runtime/startup_handoff:architecture_check_data",
    "//integration_tests/runtime/terminal_query_fixture:architecture_check_data",
]
