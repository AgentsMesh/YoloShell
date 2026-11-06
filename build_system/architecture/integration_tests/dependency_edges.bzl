"""Adversarial boundary tests using the production queries and real Bazel edges."""

load(
    "//build_system/architecture:topology.bzl",
    "CAPNP_CODEC_TARGET",
    "YOLO_BROKER_STARTUP",
    "YOLO_CLIENT",
    "YOLO_HOST",
    "YOLO_HUB_LOCAL_BACKEND",
    "YOLO_PROCESS",
    "YOLO_RELAY",
    "YOLO_REMOTE",
    "YOLO_REMOTE_BROKER",
    "YOLO_REMOTE_CLI",
    "YOLO_WORKER",
)

load("//build_system/architecture:broker_boundaries.bzl", "broker_forbidden_dependencies")
load("//build_system/architecture:hub_boundaries.bzl", "public_forbidden_dependencies")
load("//build_system/architecture:runtime_dependencies.bzl", "RUNTIME_QUERY_OPTIONS")
load("//build_system/architecture/test_support:dependency_graph.bzl", "dependency_graph", "implicit_runtime_graph")

_PACKAGE = "//build_system/architecture:"
_GENERATOR = "//build_system/bazel/capnp/generator:capnpc-rust"
_PROCESS = YOLO_PROCESS
_CLI = YOLO_REMOTE_CLI
_CODEC = CAPNP_CODEC_TARGET
_STARTUP = YOLO_BROKER_STARTUP
_CLAP = "@crates//:clap"
_HOST = YOLO_HOST
_WORKER = YOLO_WORKER
_HOST_CLI = "//core/crates/application/host/cli:yolo_host_cli"
_ATTACH_CLI = "//core/crates/application/attach/cli:yolo_attach_cli"
_TRANSPORT_CLIENTS = [
    YOLO_CLIENT,
    YOLO_RELAY,
    YOLO_REMOTE,
    YOLO_REMOTE_BROKER,
    YOLO_HUB_LOCAL_BACKEND,
]

def _case(name, expression, roots, rejected):
    query = name + "_graph"

    # Compare a stable local witness. The rejected dependency may be an
    # external alias whose printed canonical repository differs by platform.
    witness = roots[0]
    native.genquery(
        name = query,
        testonly = True,
        expression = "rdeps(deps(set({})), ({})) intersect set({})".format(
            " ".join(roots),
            expression,
            witness,
        ),
        opts = RUNTIME_QUERY_OPTIONS,
        scope = roots,
    )
    native.sh_test(
        name = name,
        srcs = ["integration_tests/dependency_edges.sh"],
        args = ["$(location :{})".format(query), "reject" if rejected else "allow", witness],
        data = [":" + query],
    )
    return ":" + name

def _public(name, rejected, **attrs):
    fixture = name + "_fixture"
    root = _PACKAGE + fixture
    dependency_graph(name = fixture, testonly = True, **attrs)
    return _case(
        name,
        "({}) except set({})".format(public_forbidden_dependencies([root]), root),
        [root],
        rejected,
    )

def _broker(name, dependencies, allowed = []):
    fixture = name + "_fixture"
    root = _PACKAGE + fixture
    dependency_graph(name = fixture, testonly = True, deps = dependencies)
    return _case(name, broker_forbidden_dependencies({root: allowed}, [root]), [root], True)

def _absent(name, roots, forbidden):
    """Asserts that no root reaches any forbidden dependency."""
    query = name + "_graph"
    native.genquery(
        name = query,
        testonly = True,
        expression = "rdeps(deps(set({})), set({})) intersect set({})".format(
            " ".join(roots),
            " ".join(forbidden),
            " ".join(roots),
        ),
        opts = RUNTIME_QUERY_OPTIONS,
        scope = roots + forbidden,
    )
    native.sh_test(
        name = name,
        srcs = ["integration_tests/dependency_edges.sh"],
        args = ["$(location :{})".format(query), "allow", roots[0]],
        data = [":" + query],
    )
    return ":" + name

def dependency_edge_tests():
    tests = [
        _public("public_exec_tool_allowed", False, tool = _GENERATOR),
        _public("public_runtime_tool_rejected", True, deps = [_GENERATOR]),
        _public("public_data_tool_rejected", True, data = [_GENERATOR]),
        _public("public_dual_tool_rejected", True, deps = [_GENERATOR], tool = _GENERATOR),
        _public("public_platform_rejected", True, deps = [_PROCESS]),
        _public("public_cli_rejected", True, deps = [_CLI]),
        _public("public_schema_codec_allowed", False, deps = [_CODEC]),
        _broker("broker_reverse_domain_rejected", [_STARTUP]),
        _broker("broker_codec_outside_supervision_rejected", [_CODEC]),
        _broker("broker_cli_leak_rejected", [_CLI], allowed = [_CLI]),
        _broker("broker_clap_leak_rejected", [_CLAP]),
        _absent("worker_does_not_depend_on_host", [_WORKER], [_HOST]),
        _absent("runtime_does_not_depend_on_role_clis", [_HOST, _WORKER], [_HOST_CLI, _ATTACH_CLI]),
        _absent("runtime_does_not_depend_on_broker_startup", [_HOST, _WORKER], [_STARTUP]),
        _absent("runtime_does_not_depend_on_transport_clients", [_HOST, _WORKER], _TRANSPORT_CLIENTS),
    ]
    fixture = "public_implicit_runtime_fixture"
    root = _PACKAGE + fixture
    implicit_runtime_graph(name = fixture, testonly = True)
    tests.append(_case(
        "public_implicit_runtime_rejected",
        "({}) except set({})".format(public_forbidden_dependencies([root]), root),
        [root],
        True,
    ))
    native.test_suite(name = "dependency_edge_tests", tests = tests)
