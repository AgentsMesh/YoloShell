"""Bazel components for the grok provider."""

GROK = struct(
    name = "grok",
    component = "agent/providers/grok",
    component_name = "yolo_grok",
    adapter = "//core/crates/agent/providers/grok:yolo_grok",
    identity = "//core/crates/agent/providers/grok/provider:yolo_grok_identity",
    provider = "//core/crates/agent/providers/grok/provider:yolo_grok_provider",
    hooks = "//core/crates/agent/providers/grok/hooks:yolo_grok_hooks",
    hook_contract = None,
    bridge = None,
    conversation = "//core/crates/agent/providers/grok/parser:yolo_conversation_grok",
    runtime_components = [
        "//core/crates/agent/providers/grok/hooks:yolo_grok_hooks",
    ],
    worker_components = [
        "//core/crates/agent/providers/grok/hooks:yolo_grok_hooks",
    ],
    runtime_tests = [
        "//core/crates/agent/providers/grok/provider:unit_tests",
        "//core/crates/agent/providers/grok:unit_tests",
        "//core/crates/agent/providers/grok/hooks:unit_tests",
    ],
    client_tests = [
        "//core/crates/agent/providers/grok/parser:unit_tests",
    ],
    hook_boundary = [
        "//core/crates/agent/providers/grok/hooks:yolo_grok_hooks",
    ],
    architecture_agent = [
        "//core/crates/agent/providers/grok/provider:architecture_check_data",
        "//core/crates/agent/providers/grok:architecture_check_data",
        "//core/crates/agent/providers/grok/hooks:architecture_check_data",
    ],
    architecture_conversation = [
        "//core/crates/agent/providers/grok/parser:architecture_check_data",
    ],
    provider_crate_deps = [
        "//core/crates/agent/providers/grok/provider:yolo_grok_identity",
        "//core/crates/agent/providers/grok/provider:yolo_grok_provider",
        "//core/crates/agent/providers/grok/hooks:yolo_grok_hooks",
        "//core/crates/agent/providers/grok:yolo_grok",
    ],
    component_entries = [
        ("agent/providers/grok/provider", "yolo_grok_provider"),
        ("agent/providers/grok", "yolo_grok"),
        ("agent/providers/grok/hooks", "yolo_grok_hooks"),
    ],
    topology_labels = [
        "//core/crates/agent/providers/grok/provider:yolo_grok_identity",
        "//core/crates/agent/providers/grok/provider:yolo_grok_provider",
        "//core/crates/agent/providers/grok:yolo_grok",
        "//core/crates/agent/providers/grok/hooks:yolo_grok_hooks",
    ],
)
