
load(
    "//build_system/architecture:topology.bzl",
    "YOLO_APPLICATION_SUPPORT",
    "YOLO_APPLICATION_TEST_FIXTURES",
    "YOLO_CLAUDE_CUSTODY",
    "YOLO_CLI",
    "YOLO_CRYPTO",
    "YOLO_LOCAL_UNIX",
    "YOLO_PATHS",
    "YOLO_PERSISTENCE_CUSTODY",
    "YOLO_PERSISTENCE_CUSTODY_RESILIENCE_SUPPORT",
    "YOLO_PROCESS",
    "YOLO_PROCESS_ADVERSARIAL",
    "YOLO_PTY_CUSTODY",
    "YOLO_SESSION_TIMING",
    "YOLO_TERMINAL_MODEL",
    "YOLO_WIRE",
    "YOLO_WORKER",
)
load("//build_system/bazel/rust:component_test_graph.bzl", "component_test_aliases", "component_test_dependencies")

PERSISTENCE_CUSTODY_COMPONENT = YOLO_PERSISTENCE_CUSTODY
PERSISTENCE_CUSTODY_RESILIENCE_COMPONENT = YOLO_PERSISTENCE_CUSTODY_RESILIENCE_SUPPORT
PROCESS_COMPONENT = YOLO_PROCESS
PROCESS_ADVERSARIAL_COMPONENT = YOLO_PROCESS_ADVERSARIAL
CLAUDE_CUSTODY_COMPONENT = YOLO_CLAUDE_CUSTODY
CLAUDE_CUSTODY_RESILIENCE_COMPONENT = "//core/crates/agent/system/custody:yolo_claude_custody_resilience_support"

_APPLICATION_SUPPORT = YOLO_APPLICATION_SUPPORT
CUSTODY_GUARDIAN_COVERAGE_SUPPORT = "//build_system/coverage/profile_test_support:library"

CUSTODY_GUARDIAN_DEPS = [
    CLAUDE_CUSTODY_COMPONENT,
    YOLO_CLI,
    _APPLICATION_SUPPORT,
    YOLO_CRYPTO,
    PERSISTENCE_CUSTODY_COMPONENT,
    YOLO_PATHS,
    PROCESS_COMPONENT,
    YOLO_PTY_CUSTODY,
    YOLO_WORKER,
    YOLO_TERMINAL_MODEL,
    YOLO_LOCAL_UNIX,
    YOLO_SESSION_TIMING,
    YOLO_WIRE,
    "@crates//:anyhow",
    "@crates//:libc",
    "@crates//:serde",
    "@crates//:tokio",
    "@crates//:tracing",
    "@crates//:uuid",
]

_TEST_DEPS = CUSTODY_GUARDIAN_DEPS + [
    CUSTODY_GUARDIAN_COVERAGE_SUPPORT,
    YOLO_APPLICATION_TEST_FIXTURES,
    "@crates//:futures",
    "@crates//:portable-pty",
    "@crates//:serde_json",
    "@crates//:tempfile",
]

CUSTODY_GUARDIAN_TEST_ALIASES = component_test_aliases(_TEST_DEPS)

CUSTODY_GUARDIAN_TEST_DEPS = component_test_dependencies(_TEST_DEPS)
