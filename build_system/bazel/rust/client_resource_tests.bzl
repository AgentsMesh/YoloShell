"""Public client extension, conversation and presentation contract acceptance."""

load("//build_system/bazel/rust:providers.bzl", "PROVIDER_CLIENT_TESTS")

CLIENT_RESOURCE_TESTS = [
    "//clients/tui/chat:unit_tests",
    "//clients/tui/chat:integration_tests",
    "//core/crates/hub/conversation/client:integration_tests",
    "//core/crates/agent/cli:unit_tests",
    "//core/crates/agent/cli:integration_tests",
    "//core/crates/conversation/cli:unit_tests",
    "//core/crates/conversation/cli:integration_tests",
    "//core/crates/presentation/layout:unit_tests",
    "//core/crates/presentation/layout:integration_tests",
    "//core/crates/agent/input:unit_tests",
    "//core/crates/conversation/protocol:unit_tests",
    "//core/crates/conversation/transcript:unit_tests",
    "//core/crates/conversation/service:unit_tests",
    "//core/crates/conversation/source:unit_tests",
    "//core/crates/conversation/service:integration_tests",
    "//clients/tui/descriptor:unit_tests",
    "//core/crates/application/protocol/cli:unit_tests",
    "//core/crates/hub/rpc/descriptor:unit_tests",
    "//core/crates/presentation/protocol:unit_tests",
    "//core/crates/presentation/protocol:integration_tests",
    "//core/crates/presentation/store:unit_tests",
    "//core/crates/presentation/store:integration_tests",
] + PROVIDER_CLIENT_TESTS
