"""Reviewed aggregate dependencies not yet referenced by their target sources.

Remove an entry after using the dependency or splitting the shared list; do not
add entries for new dependencies.
"""

DEPENDENCY_USAGE_BASELINE = [
    ("build_system/architecture/checks/process/signal/policy/BUILD.bazel", "library", "//build_system/architecture/graph:library"),
    ("build_system/release/developer_release_system/BUILD.bazel", "library", "@crates//:tempfile"),
    ("build_system/release/gate_system/BUILD.bazel", "library", "@crates//:tempfile"),
    ("build_system/release/linux_gate_system/BUILD.bazel", "library", "@crates//:tempfile"),
    ("build_system/release/verification/BUILD.bazel", "library", "@crates//:ring"),
    ("core/crates/agent/providers/codex/hooks/BUILD.bazel", "yolo_codex_hooks", "@crates//:tracing"),
    ("core/crates/agent/system/hooks/runtime/BUILD.bazel", "yolo_hooks_runtime", "@crates//:serde_json"),
    ("core/crates/application/broker/child/custody/BUILD.bazel", "yolo_broker_child_custody", "@crates//:tokio"),
    ("core/crates/application/broker/startup/BUILD.bazel", "yolo_broker_startup", "//core/crates/runtime/engine/process:yolo_process"),
    ("core/crates/application/broker/supervision/BUILD.bazel", "yolo_broker_supervision", "@crates//:serde_json"),
    ("core/crates/application/remote/cli/BUILD.bazel", "yolo_remote_cli", "@crates//:serde_json"),
    ("core/crates/application/support/BUILD.bazel", "yolo_application_support", "@crates//:libc"),
    ("core/crates/distribution/release/store/BUILD.bazel", "yolo_distribution_release_store", "@crates//:serde"),
    ("core/crates/hub/attach/cli/BUILD.bazel", "yolo_hub_attach_cli", "@crates//:serde"),
    ("core/crates/hub/attach/cli/BUILD.bazel", "yolo_hub_attach_cli", "@crates//:serde_json"),
    ("core/crates/hub/connector/client/BUILD.bazel", "yolo_hub_connector_client", "//core/crates/hub/jsonrpc/protocol:yolo_hub_jsonrpc_protocol"),
    ("core/crates/hub/enrollment/store/BUILD.bazel", "yolo_hub_enrollment_store", "//core/crates/hub/federation/settings:yolo_hub_federation_settings"),
    ("core/crates/hub/federation/directory/BUILD.bazel", "yolo_hub_federation_directory", "//core/crates/hub/terminal/protocol:yolo_hub_terminal_protocol"),
    ("core/crates/hub/local/startup/BUILD.bazel", "yolo_hub_local_startup", "@crates//:serde_json"),
    ("core/crates/hub/peer/protocol/BUILD.bazel", "yolo_hub_peer_protocol", "//core/crates/hub/transport:yolo_hub_transport"),
    ("core/crates/hub/relay/control/protocol/BUILD.bazel", "yolo_hub_relay_control_protocol", "@crates//:serde_json"),
    ("core/crates/hub/remote/setup/runtime/BUILD.bazel", "yolo_hub_remote_setup_runtime", "//core/crates/hub/delegation:yolo_hub_delegation"),
    ("core/crates/hub/server/BUILD.bazel", "yolo_hub_server", "//core/crates/hub/identity:yolo_hub_identity"),
    ("core/crates/hub/ssh/entry/BUILD.bazel", "yolo_hub_ssh_entry", "//core/crates/hub/jsonrpc/protocol:yolo_hub_jsonrpc_protocol"),
    ("core/crates/hub/terminal/client/BUILD.bazel", "yolo_hub_terminal_client", "//core/crates/hub/jsonrpc/protocol:yolo_hub_jsonrpc_protocol"),
    ("core/crates/persistence/durable/reboot/BUILD.bazel", "yolo_durable_reboot", "@crates//:serde_json"),
    ("core/crates/plugin/client/host/BUILD.bazel", "yolo_builtin_client_host", "//core/crates/hub/identity:yolo_hub_identity"),
    ("core/crates/transport/broker/launch/authority/BUILD.bazel", "yolo_broker_launch_authority", "@crates//:serde_json"),
    ("core/crates/transport/remote/BUILD.bazel", "yolo_remote", "//core/crates/transport/capnp/codec"),
    ("core/crates/transport/remote/BUILD.bazel", "yolo_remote", "//core/crates/transport/capnp/rpc"),
    ("core/crates/transport/remote/provisioning/BUILD.bazel", "yolo_remote_provisioning", "@crates//:serde_json"),
    ("clients/tui/app/BUILD.bazel", "yolo_tui_app", "//core/crates/crypto:yolo_crypto"),
    ("clients/tui/app/BUILD.bazel", "yolo_tui_app", "//core/crates/hub/catalog/client:yolo_hub_catalog_client"),
    ("clients/tui/chat/BUILD.bazel", "yolo_tui_chat", "//core/crates/crypto:yolo_crypto"),
    ("clients/tui/creation/recovery/BUILD.bazel", "yolo_tui_creation_recovery", "//core/crates/hub/identity:yolo_hub_identity"),
    ("clients/tui/local/startup/BUILD.bazel", "yolo_tui_local_startup", "@crates//:serde_json"),
]
