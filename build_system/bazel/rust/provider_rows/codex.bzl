"""Bazel components for the codex provider."""

CODEX = struct(
    name = "codex",
    component = "agent/providers/codex",
    component_name = "yolo_codex",
    adapter = "//core/crates/agent/providers/codex:yolo_codex",
    identity = "//core/crates/agent/providers/codex/provider:yolo_codex_identity",
    provider = "//core/crates/agent/providers/codex/provider:yolo_codex_provider",
    hooks = "//core/crates/agent/providers/codex/hooks:yolo_codex_hooks",
    hook_contract = "//core/crates/agent/providers/codex/hooks/contract:yolo_codex_hook_contract",
    bridge = None,
    conversation = "//core/crates/agent/providers/codex/parser:yolo_conversation_codex",
    runtime_components = [
        "//core/crates/agent/providers/codex:yolo_codex",
        "//core/crates/agent/providers/codex/hooks:yolo_codex_hooks",
    ],
    worker_components = [
        "//core/crates/agent/providers/codex:yolo_codex",
        "//core/crates/agent/providers/codex/hooks:yolo_codex_hooks",
    ],
    runtime_tests = [
        "//core/crates/agent/providers/codex/provider:unit_tests",
        "//core/crates/agent/providers/codex:yolo_codex_unit_tests",
        "//core/crates/agent/providers/codex/hooks:yolo_codex_hooks_integration_tests",
        "//core/crates/agent/providers/codex/hooks:yolo_codex_hooks_unit_tests",
    ],
    client_tests = [
        "//core/crates/agent/providers/codex/parser:unit_tests",
    ],
    hook_boundary = [
        "//core/crates/agent/providers/codex/hooks:yolo_codex_hooks",
        "//core/crates/agent/providers/codex/hooks/contract:yolo_codex_hook_contract",
    ],
    architecture_agent = [
        "//core/crates/agent/providers/codex/provider:architecture_check_data",
        "//core/crates/agent/providers/codex:architecture_check_data",
        "//core/crates/agent/providers/codex/hooks/contract:architecture_check_data",
        "//core/crates/agent/providers/codex/hooks:architecture_check_data",
    ],
    architecture_conversation = [
        "//core/crates/agent/providers/codex/parser:architecture_check_data",
    ],
    provider_crate_deps = [
        "//core/crates/agent/providers/codex/provider:yolo_codex_identity",
        "//core/crates/agent/providers/codex/provider:yolo_codex_provider",
        "//core/crates/agent/providers/codex:yolo_codex",
        "//core/crates/agent/providers/codex/hooks/contract:yolo_codex_hook_contract",
    ],
    component_entries = [
        ("agent/providers/codex/provider", "yolo_codex_provider"),
        ("agent/providers/codex", "yolo_codex"),
        ("agent/providers/codex/hooks", "yolo_codex_hooks"),
    ],
    topology_labels = [
        "//core/crates/agent/providers/codex/provider:yolo_codex_identity",
        "//core/crates/agent/providers/codex/provider:yolo_codex_provider",
        "//core/crates/agent/providers/codex:yolo_codex",
        "//core/crates/agent/providers/codex/hooks:yolo_codex_hooks",
        "//core/crates/agent/providers/codex/hooks/contract:yolo_codex_hook_contract",
    ],
)
