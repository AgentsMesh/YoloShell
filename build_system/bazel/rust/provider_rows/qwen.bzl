"""Bazel components for the qwen provider."""

QWEN = struct(
    name = "qwen",
    component = "agent/providers/qwen",
    component_name = "yolo_qwen",
    adapter = "//core/crates/agent/providers/qwen:yolo_qwen",
    identity = "//core/crates/agent/providers/qwen/provider:yolo_qwen_identity",
    provider = "//core/crates/agent/providers/qwen/provider:yolo_qwen_provider",
    hooks = "//core/crates/agent/providers/qwen/hooks:yolo_qwen_hooks",
    hook_contract = None,
    bridge = None,
    conversation = "//core/crates/agent/providers/qwen/parser:yolo_conversation_qwen",
    runtime_components = [
        "//core/crates/agent/providers/qwen/hooks:yolo_qwen_hooks",
    ],
    worker_components = [
        "//core/crates/agent/providers/qwen:yolo_qwen",
        "//core/crates/agent/providers/qwen/hooks:yolo_qwen_hooks",
    ],
    runtime_tests = [
        "//core/crates/agent/providers/qwen/provider:unit_tests",
        "//core/crates/agent/providers/qwen:unit_tests",
        "//core/crates/agent/providers/qwen/hooks:unit_tests",
    ],
    client_tests = [
        "//core/crates/agent/providers/qwen/parser:unit_tests",
    ],
    hook_boundary = [
        "//core/crates/agent/providers/qwen/hooks:yolo_qwen_hooks",
    ],
    architecture_agent = [
        "//core/crates/agent/providers/qwen/provider:architecture_check_data",
        "//core/crates/agent/providers/qwen:architecture_check_data",
        "//core/crates/agent/providers/qwen/hooks:architecture_check_data",
    ],
    architecture_conversation = [
        "//core/crates/agent/providers/qwen/parser:architecture_check_data",
    ],
    provider_crate_deps = [
        "//core/crates/agent/providers/qwen/provider:yolo_qwen_identity",
        "//core/crates/agent/providers/qwen/provider:yolo_qwen_provider",
        "//core/crates/agent/providers/qwen:yolo_qwen",
        "//core/crates/agent/providers/qwen/hooks:yolo_qwen_hooks",
    ],
    component_entries = [
        ("agent/providers/qwen/provider", "yolo_qwen_provider"),
        ("agent/providers/qwen", "yolo_qwen"),
        ("agent/providers/qwen/hooks", "yolo_qwen_hooks"),
    ],
    topology_labels = [
        "//core/crates/agent/providers/qwen/provider:yolo_qwen_identity",
        "//core/crates/agent/providers/qwen/provider:yolo_qwen_provider",
        "//core/crates/agent/providers/qwen:yolo_qwen",
        "//core/crates/agent/providers/qwen/hooks:yolo_qwen_hooks",
    ],
)
