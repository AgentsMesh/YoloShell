"""Sources and dependencies for the relay runtime-dimension unit target."""

load(
    "//build_system/architecture:topology.bzl",
    "YOLO_RELAY",
)

_PROCESS_SUPPORT = "//core/crates/runtime/engine/process:yolo_process_component_test_support"

RELAY_DIMENSIONS_UNIT_SRCS = [
    "resilience/relay_dimensions.rs",
    "resilience/relay/timing.rs",
    "resilience/relay/test_support/dimensions.rs",
    "resilience/relay/unit_tests/dimensions.rs",
    "resilience/relay/unit_tests/lib.rs",
]

RELAY_DIMENSIONS_UNIT_ALIASES = {_PROCESS_SUPPORT: "yolo_process_test_support"}

RELAY_DIMENSIONS_UNIT_DEPS = [
    "//core:yolo_runtime",
    _PROCESS_SUPPORT,
    YOLO_RELAY,
    "@crates//:anyhow",
    "@crates//:serde_json",
    "@crates//:tempfile",
    "@crates//:tokio",
    "@crates//:uuid",
]
