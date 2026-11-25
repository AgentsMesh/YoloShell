"""Broker orchestration keeps one-way component edges and no parser dependencies."""

load(
    "//build_system/architecture:topology.bzl",
    "CAPNP_CODEC_TARGET",
    "YOLO_BROKER_HUB_PROTOCOL",
    "YOLO_BROKER_STARTUP",
    "YOLO_CLI",
    "YOLO_LAUNCH_CONTRACT",
    "YOLO_LOCAL_UNIX",
    "YOLO_PROCESS",
    "YOLO_REMOTE_BROKER",
    "YOLO_REMOTE_CLI",
    "YOLO_WIRE",
)

load(":runtime_dependencies.bzl", "RUNTIME_QUERY_OPTIONS")

_PROFILE = "//core/crates/application/broker/profile/authority:yolo_broker_profile_authority"
_STATE = "//core/crates/application/broker/supervision/state:yolo_broker_supervision_state"
_CUSTODY = "//core/crates/application/broker/child/custody:yolo_broker_child_custody"
_SUPERVISION = "//core/crates/application/broker/supervision:yolo_broker_supervision"
_STARTUP = YOLO_BROKER_STARTUP
_CLI = YOLO_REMOTE_CLI
_PROCESS = YOLO_PROCESS
_BROKER = YOLO_REMOTE_BROKER
_AUTHORITY = "//core/crates/transport/broker/launch/authority:yolo_broker_launch_authority"
_LAUNCH = YOLO_LAUNCH_CONTRACT

# application/cli contains only command constants and reexports the pure launch contract.
_COMMAND_CONTRACT = YOLO_CLI

_ALLOWED_LOCAL = {
    _PROFILE: [YOLO_WIRE],
    _STATE: ["//core/crates/persistence/paths/core:yolo_paths_core", _PROCESS, _BROKER],
    _CUSTODY: [_PROCESS, _STATE, _BROKER],
    _SUPERVISION: [
        _COMMAND_CONTRACT,
        _PROCESS,
        CAPNP_CODEC_TARGET,
        YOLO_LOCAL_UNIX,
        _CUSTODY,
        _STATE,
        _BROKER,
    ],
    _STARTUP: [
        _PROFILE,
        _CUSTODY,
        _SUPERVISION,
        _STATE,
        _PROCESS,
        _AUTHORITY,
        YOLO_BROKER_HUB_PROTOCOL,
        _BROKER,
    ],
    _CLI: [
        _STARTUP,
        _SUPERVISION,
        _STATE,
        _LAUNCH,
        _AUTHORITY,
        _COMMAND_CONTRACT,
        _PROCESS,
        _BROKER,
        CAPNP_CODEC_TARGET,
    ],
}

_DOMAIN_ROOTS = [_PROFILE, _STATE, _CUSTODY, _SUPERVISION, _STARTUP]
_ENTRY_PACKAGES = "^//core/crates/(application/(remote/cli|host/cli|attach/cli|yolo)|hub/(cli|desktop/cli|attach/cli|file/helper)|tui/cli):"
_CLAP_PACKAGES = "(^|[_+/:])clap([_+.:/-]|$)"

def broker_forbidden_dependencies(allowed_local, domain_roots):
    """Check exact local edges plus transitive entry-point/parser leakage."""
    queries = [
        "(filter('^//', labels(deps, {})) except set({}))".format(target, " ".join(allowed))
        for target, allowed in allowed_local.items()
    ]
    domains = "deps(set({}))".format(" ".join(domain_roots))
    queries.extend([
        "filter('{}', {})".format(_ENTRY_PACKAGES, domains),
        "filter('{}', {})".format(_CLAP_PACKAGES, domains),
    ])
    return " union ".join(queries)

def broker_dependency_boundary_test():
    scope = _ALLOWED_LOCAL.keys() + [target for allowed in _ALLOWED_LOCAL.values() for target in allowed]
    native.genquery(
        name = "broker_forbidden_dependencies",
        expression = broker_forbidden_dependencies(_ALLOWED_LOCAL, _DOMAIN_ROOTS),
        opts = RUNTIME_QUERY_OPTIONS,
        scope = depset(scope).to_list(),
    )
    native.sh_test(
        name = "broker_dependency_boundary_test",
        srcs = ["integration_tests/broker_dependency_boundary.sh"],
        args = ["$(location :broker_forbidden_dependencies)"],
        data = [":broker_forbidden_dependencies"],
    )
    components = []
    for target, allowed in _ALLOWED_LOCAL.items():
        name = target.split(":")[-1] + "_boundary_test"
        query = name + "_dependencies"
        native.genquery(
            name = query,
            expression = broker_forbidden_dependencies(
                {target: allowed},
                [target] if target in _DOMAIN_ROOTS else [],
            ),
            opts = RUNTIME_QUERY_OPTIONS,
            scope = [target] + allowed,
        )
        native.sh_test(
            name = name,
            srcs = ["integration_tests/broker_dependency_boundary.sh"],
            args = ["$(location :{})".format(query)],
            data = [":" + query],
        )
        components.append(":" + name)
    native.test_suite(name = "broker_component_boundary_tests", tests = components)
