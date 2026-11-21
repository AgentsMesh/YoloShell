"""Shared inputs for suites using the runtime resilience Host and PTY harness."""

load(":config.bzl", "RUNTIME_DEPS")
load(":runtime_hub_integration.bzl", "HUB_BUNDLE_TEST_OBJECTS", "HUB_CONNECTOR_TEST_OBJECTS", "HUB_FILE_TEST_OBJECTS", "HUB_INTEGRATION_DEPS", "HUB_QUERY_TEST_OBJECTS", "native_hub_environment")

RESILIENCE_ALIASES = {
    "//core/crates/runtime/engine/process:yolo_process_component_test_support": "yolo_process_test_support",
    "//core/crates/transport/client:yolo_client_adversarial": "yolo_client",
    "//core/crates/transport/relay:yolo_relay_adversarial": "yolo_relay",
    "//core/crates/transport/remote:yolo_remote_adversarial": "yolo_remote",
    "//core/crates/transport/remote/broker:yolo_remote_broker_adversarial": "yolo_remote_broker",
}

RESILIENCE_COVERAGE = depset([
    "//core/crates/runtime/engine/startup/guard:startup_parent",
    ":yolo-test-claude",
    ":yolo-test-codex",
    "//core/crates/application/yolo:yolo",
    "//core:yolo-adversarial",
] + HUB_FILE_TEST_OBJECTS + HUB_CONNECTOR_TEST_OBJECTS + HUB_QUERY_TEST_OBJECTS + HUB_BUNDLE_TEST_OBJECTS).to_list()

RESILIENCE_DEPS = RUNTIME_DEPS + HUB_INTEGRATION_DEPS + [
    "//core/crates/conversation/protocol:yolo_conversation_protocol",
    "//core/crates/hub/conversation/client:yolo_hub_conversation_client",
    "//build_system/coverage/profile_test_support:library",
    "//core:yolo_runtime_adversarial",
    "//core/crates/agent/system/core:yolo_agent_core",
    "//core/crates/agent/system/hooks/installation:yolo_hook_installation",
    "//core/crates/runtime/engine/process:yolo_process_component_test_support",
    "//core/crates/runtime/engine/resilience/support:yolo_resilience_support",
    "//core/crates/transport/client:yolo_client_adversarial",
    "//core/crates/transport/relay:yolo_relay_adversarial",
    "//core/crates/transport/remote:yolo_remote_adversarial",
    "//core/crates/transport/remote/broker:yolo_remote_broker_adversarial",
    "//core/crates/transport/session:yolo_session_timing",
    "@crates//:tempfile",
]

RESILIENCE_ENV = native_hub_environment({
    "RUST_TEST_THREADS": "1",
    "YOLO_ATTACH_BINARY": "$(rootpath //core/crates/application/yolo:yolo)",
    "YOLO_TEST_RUNTIME": "$(rootpath //core:yolo-adversarial)",
    "YOLO_TEST_TUI_BINARY": "$(rootpath //core/crates/application/yolo:yolo)",
    "YOLO_TEST_HUB_ATTACH_BINARY": "$(rootpath //core/crates/application/yolo:yolo)",
    "YOLO_TEST_HUB_BINARY": "$(rootpath //core/crates/application/yolo:yolo)",
})

RESILIENCE_BINARIES = [
    ":yolo-test-claude",
    ":yolo-test-codex",
    "//core/crates/application/yolo:yolo",
    "//core:yolo-adversarial",
]
