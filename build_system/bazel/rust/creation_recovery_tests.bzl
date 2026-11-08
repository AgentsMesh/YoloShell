"""Fixture-only contracts for the TUI creation-recovery proxy."""

load(
    "//build_system/architecture:topology.bzl",
    "YOLO_HUB_IDENTITY",
    "YOLO_HUB_RPC_PROTOCOL",
    "YOLO_PATHS",
    "YOLO_PRESENTATION_LAYOUT",
    "YOLO_PRESENTATION_PROTOCOL",
    "YOLO_TUI_TAB",
)

CREATION_RECOVERY_AUDIT_UNIT_SRCS = [
    "resilience/hub/tui_creation/tui_creation_recovery/test_support/layout_audit.rs",
    "resilience/hub/tui_creation/tui_creation_recovery/unit_tests/lib.rs",
    "resilience/hub/tui_creation/tui_creation_recovery/unit_tests/layout_audit.rs",
]

CREATION_RECOVERY_AUDIT_UNIT_DEPS = [
    YOLO_HUB_IDENTITY,
    YOLO_HUB_RPC_PROTOCOL,
    YOLO_PATHS,
    YOLO_PRESENTATION_LAYOUT,
    YOLO_PRESENTATION_PROTOCOL,
    YOLO_TUI_TAB,
    "@crates//:tempfile",
    "@crates//:uuid",
]
