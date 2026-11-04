"""Bazel components for the gemini provider."""

GEMINI = struct(
    name = "gemini",
    component = "agent/providers/gemini",
    component_name = "yolo_gemini",
    adapter = "//core/crates/agent/providers/gemini:yolo_gemini",
    identity = "//core/crates/agent/providers/gemini/provider:yolo_gemini_identity",
    provider = "//core/crates/agent/providers/gemini/provider:yolo_gemini_provider",
    hooks = "//core/crates/agent/providers/gemini/hooks:yolo_gemini_hooks",
    hook_contract = None,
    bridge = None,
    conversation = "//core/crates/agent/providers/gemini/parser:yolo_conversation_gemini",
    runtime_components = [
        "//core/crates/agent/providers/gemini/hooks:yolo_gemini_hooks",
    ],
    worker_components = [
        "//core/crates/agent/providers/gemini/hooks:yolo_gemini_hooks",
    ],
    runtime_tests = [
        "//core/crates/agent/providers/gemini/provider:unit_tests",
        "//core/crates/agent/providers/gemini:unit_tests",
        "//core/crates/agent/providers/gemini/hooks:unit_tests",
    ],
    client_tests = [
        "//core/crates/agent/providers/gemini/parser:unit_tests",
    ],
    hook_boundary = [
        "//core/crates/agent/providers/gemini/hooks:yolo_gemini_hooks",
    ],
    architecture_agent = [
        "//core/crates/agent/providers/gemini/provider:architecture_check_data",
        "//core/crates/agent/providers/gemini:architecture_check_data",
        "//core/crates/agent/providers/gemini/hooks:architecture_check_data",
    ],
    architecture_conversation = [
        "//core/crates/agent/providers/gemini/parser:architecture_check_data",
    ],
    provider_crate_deps = [
        "//core/crates/agent/providers/gemini/provider:yolo_gemini_identity",
        "//core/crates/agent/providers/gemini/provider:yolo_gemini_provider",
        "//core/crates/agent/providers/gemini/hooks:yolo_gemini_hooks",
        "//core/crates/agent/providers/gemini:yolo_gemini",
    ],
    component_entries = [
        ("agent/providers/gemini/provider", "yolo_gemini_provider"),
        ("agent/providers/gemini", "yolo_gemini"),
        ("agent/providers/gemini/hooks", "yolo_gemini_hooks"),
    ],
    topology_labels = [
        "//core/crates/agent/providers/gemini/provider:yolo_gemini_identity",
        "//core/crates/agent/providers/gemini/provider:yolo_gemini_provider",
        "//core/crates/agent/providers/gemini:yolo_gemini",
        "//core/crates/agent/providers/gemini/hooks:yolo_gemini_hooks",
    ],
)
