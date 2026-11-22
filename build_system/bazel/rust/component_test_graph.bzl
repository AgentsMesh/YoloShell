"""Consistent dependency variants for cross-component runtime tests."""

load("//build_system/bazel/rust:providers.bzl", "PROVIDER_COMPONENT_ENTRIES")

_COMPONENT_PREFIX = "//core/crates/"

_TEST_VARIANT_SUFFIX = "_component_test_support"

_BASE_COMPONENT_ENTRIES = [
    ("agent/system/authority/generation", "yolo_agent_hook_generation"),
    ("agent/system/core", "yolo_agent_core"),
    ("agent/system/hooks/lineage", "yolo_hook_lineage"),
    ("agent/system/hooks/ingress", "yolo_hook_ingress"),
    ("agent/system/control/journal", "yolo_agent_control_journal"),
    ("agent/system/control/runtime", "yolo_agent_control_runtime"),
    ("agent/system/custody", "yolo_claude_custody"),
    ("agent/system/detection", "yolo_agent_detection"),
    ("agent/system/evidence", "yolo_agent_evidence"),
    ("agent/system/launch", "yolo_agent_launch"),
    ("agent/system/process", "yolo_agent_process"),
    ("agent/system/providers", "yolo_agent_providers"),
    ("agent/system/store", "yolo_agent_store"),
    ("persistence/catalog", "yolo_catalog"),
    ("persistence/custody", "yolo_persistence_custody"),
    ("persistence/durable/reboot", "yolo_durable_reboot"),
    ("persistence/journal", "yolo_journal"),
    ("persistence/maintenance", "yolo_maintenance"),
    ("runtime/engine/custody/guardian", "yolo_custody_guardian"),
    ("runtime/engine/host", "yolo_host"),
    ("runtime/engine/host/retained_history", "yolo_host_retained_history"),
    ("runtime/engine/host/operation/deadline", "yolo_host_operation_deadline"),
    ("runtime/engine/process", "yolo_process"),
    ("runtime/engine/pty/custody", "yolo_pty_custody"),
    ("runtime/engine/startup/checkpoint/contract", "yolo_startup_checkpoint_contract"),
    ("runtime/engine/worker", "yolo_worker"),
    ("runtime/engine/worker/launch/fence", "yolo_agent_launch_fence"),
]

_COMPONENTS = dict(_BASE_COMPONENT_ENTRIES + PROVIDER_COMPONENT_ENTRIES)

_EXISTING_VARIANTS = {
    "agent/system/binding/io:yolo_agent_binding_io": "yolo_agent_binding_io_test_support",
    "agent/system/hooks/runtime:yolo_hooks_runtime": "yolo_hooks_runtime_test_support",
    "application/support:yolo_application_support": "yolo_application_support_test_support",
    "persistence/lease:yolo_lease": "yolo_lease_test_support",
    "persistence/paths:yolo_paths": "yolo_paths_persistence_test_support",
    "process/spawn:yolo_process_spawn": "yolo_process_spawn_test_support",
    "runtime/engine/process/identity:yolo_process_identity": "yolo_process_identity_test_support",
    "transport/core:yolo_transport_core": "yolo_transport_core_test_support",
    "transport/local/unix:yolo_local_unix": "yolo_local_unix_test_support",
}

_REPLACEMENTS = {
    _COMPONENT_PREFIX + package + ":" + name: _COMPONENT_PREFIX + package + ":" + name + _TEST_VARIANT_SUFFIX
    for package, name in _COMPONENTS.items()
}

_REPLACEMENTS.update({
    _COMPONENT_PREFIX + source: _COMPONENT_PREFIX + source.split(":")[0] + ":" + target
    for source, target in _EXISTING_VARIANTS.items()
})

COMPONENT_TEST_FEATURES = ["adversarial-test-hooks", "runtime-component-tests", "yolo-test-support"]

def component_test_dependencies(dependencies):
    return [_REPLACEMENTS.get(dependency, dependency) for dependency in dependencies]

def component_test_aliases(dependencies):
    return {
        _REPLACEMENTS[dependency]: dependency.split(":")[-1]
        for dependency in dependencies
        if dependency in _REPLACEMENTS
    }
