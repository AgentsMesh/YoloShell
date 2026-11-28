"""Bazel components for the claude provider."""

CLAUDE = struct(
    name = "claude",
    component = "agent/providers/claude",
    component_name = "yolo_claude",
    adapter = "//core/crates/agent/providers/claude:yolo_claude",
    identity = "//core/crates/agent/providers/claude/provider:yolo_claude_identity",
    provider = "//core/crates/agent/providers/claude/provider:yolo_claude_provider",
    hooks = "//core/crates/agent/providers/claude/hooks:yolo_claude_hooks",
    hook_contract = "//core/crates/agent/providers/claude/hooks/contract:yolo_claude_hook_contract",
    bridge = "//core/crates/agent/providers/claude/hooks/bridge:yolo_claude_hook_bridge",
    conversation = "//core/crates/agent/providers/claude/parser:yolo_conversation_claude",
    runtime_components = [
        "//core/crates/agent/providers/claude:yolo_claude",
        "//core/crates/agent/providers/claude/hooks/bridge:yolo_claude_hook_bridge",
        "//core/crates/agent/providers/claude/hooks:yolo_claude_hooks",
    ],
    worker_components = [
        "//core/crates/agent/providers/claude:yolo_claude",
        "//core/crates/agent/providers/claude/hooks:yolo_claude_hooks",
    ],
    runtime_tests = [
        "//core/crates/agent/providers/claude/provider:unit_tests",
        "//core/crates/agent/providers/claude:yolo_claude_integration_tests",
        "//core/crates/agent/providers/claude:yolo_claude_unit_tests",
        "//core/crates/agent/providers/claude/hooks/bridge:yolo_claude_hook_bridge_integration_tests",
        "//core/crates/agent/providers/claude/hooks/bridge:yolo_claude_hook_bridge_unit_tests",
        "//core/crates/agent/providers/claude/hooks:yolo_claude_hooks_integration_tests",
        "//core/crates/agent/providers/claude/hooks:yolo_claude_hooks_unit_tests",
    ],
    client_tests = [
        "//core/crates/agent/providers/claude/parser:unit_tests",
    ],
    hook_boundary = [
        "//core/crates/agent/providers/claude/hooks:yolo_claude_hooks",
        "//core/crates/agent/providers/claude/hooks/bridge:yolo_claude_hook_bridge",
        "//core/crates/agent/providers/claude/hooks/contract:yolo_claude_hook_contract",
    ],
    architecture_agent = [
        "//core/crates/agent/providers/claude/provider:architecture_check_data",
        "//core/crates/agent/providers/claude:architecture_check_data",
        "//core/crates/agent/providers/claude/hooks/bridge:architecture_check_data",
        "//core/crates/agent/providers/claude/hooks/contract:architecture_check_data",
        "//core/crates/agent/providers/claude/hooks:architecture_check_data",
    ],
    architecture_conversation = [
        "//core/crates/agent/providers/claude/parser:architecture_check_data",
    ],
    provider_crate_deps = [
        "//core/crates/agent/providers/claude/provider:yolo_claude_identity",
        "//core/crates/agent/providers/claude/provider:yolo_claude_provider",
        "//core/crates/agent/providers/claude:yolo_claude",
        "//core/crates/agent/providers/claude/hooks/contract:yolo_claude_hook_contract",
    ],
    component_entries = [
        ("agent/providers/claude/provider", "yolo_claude_provider"),
        ("agent/providers/claude", "yolo_claude"),
        ("agent/providers/claude/hooks", "yolo_claude_hooks"),
    ],
    topology_labels = [
        "//core/crates/agent/providers/claude/provider:yolo_claude_identity",
        "//core/crates/agent/providers/claude/provider:yolo_claude_provider",
        "//core/crates/agent/providers/claude:yolo_claude",
        "//core/crates/agent/providers/claude/hooks:yolo_claude_hooks",
        "//core/crates/agent/providers/claude/hooks/contract:yolo_claude_hook_contract",
        "//core/crates/agent/providers/claude/hooks/bridge:yolo_claude_hook_bridge",
    ],
)
