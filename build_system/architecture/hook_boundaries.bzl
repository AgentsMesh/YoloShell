"""Hooks remain callable without linking the agent startup wrappers."""

load(
    "//build_system/architecture:topology.bzl",
    "YOLO_HOOK_CODEC",
    "YOLO_HOOK_INGRESS",
)
load("//build_system/bazel/rust:providers.bzl", "PROVIDER_HOOK_BOUNDARY_COMPONENTS")

_HOOK_COMPONENTS = PROVIDER_HOOK_BOUNDARY_COMPONENTS + [
    YOLO_HOOK_CODEC,
    YOLO_HOOK_INGRESS,
    "//core/crates/agent/system/hooks/installation:yolo_hook_installation",
    "//core/crates/agent/system/hooks/installation/contract:yolo_hook_installation_contract",
]
_FORBIDDEN_DEPENDENCIES = "^//core/crates/runtime/engine/(worker|host):"

def hook_dependency_boundary_test():
    native.genquery(
        name = "hook_forbidden_dependencies",
        expression = "filter('{}', deps(set({})))".format(
            _FORBIDDEN_DEPENDENCIES,
            " ".join(_HOOK_COMPONENTS),
        ),
        scope = _HOOK_COMPONENTS,
    )
    native.sh_test(
        name = "hook_dependency_boundary_test",
        srcs = ["integration_tests/hook_dependency_boundary.sh"],
        args = ["$(location :hook_forbidden_dependencies)"],
        data = [":hook_forbidden_dependencies"],
    )
