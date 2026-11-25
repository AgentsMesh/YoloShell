"""External JSON-RPC and internal capability transports have distinct owners."""

load(
    "//build_system/architecture:topology.bzl",
    "CAPNP_RPC",
    "TRANSPORT_RUNTIME_RPC",
    "YOLO_HUB_CLIENT",
    "YOLO_HUB_CONNECTOR_CLIENT",
    "YOLO_HUB_JSONRPC_PROTOCOL",
    "YOLO_HUB_JSONRPC_TRANSPORT",
)

_PUBLIC = [
    YOLO_HUB_JSONRPC_PROTOCOL,
    YOLO_HUB_JSONRPC_TRANSPORT,
    "//core/crates/hub/terminal/wire:yolo_hub_terminal_wire",
    "//core/crates/hub/connector/jsonrpc:yolo_hub_connector_jsonrpc",
    YOLO_HUB_CLIENT,
    YOLO_HUB_CONNECTOR_CLIENT,
]
_INTERNAL = [
    CAPNP_RPC,
    TRANSPORT_RUNTIME_RPC,
    "//core/crates/hub/peer/rpc",
]
_INTERNAL_PROTOCOLS = "^//core/crates/(wire|transport/(capnp/rpc|runtime)|hub/(transport|peer/rpc))(/|:)"
_PUBLIC_PROTOCOLS = "^//core/crates/hub/(jsonrpc|terminal/wire|connector/jsonrpc)(/|:)"
_RETIRED_CODECS = "rmp[-_]serde|rmpv"

def communication_dependency_boundary_test():
    public = "deps(set({}))".format(" ".join(_PUBLIC))
    internal = "deps(set({}))".format(" ".join(_INTERNAL))
    native.genquery(
        name = "communication_forbidden_dependencies",
        expression = "filter('{}', {}) union filter('{}', {}) union filter('{}', {} union {})".format(
            _INTERNAL_PROTOCOLS, public,
            _PUBLIC_PROTOCOLS, internal,
            _RETIRED_CODECS, public, internal,
        ),
        scope = _PUBLIC + _INTERNAL,
    )
    native.sh_test(
        name = "communication_dependency_boundary_test",
        srcs = ["integration_tests/communication_dependency_boundary.sh"],
        args = ["$(location :communication_forbidden_dependencies)"],
        data = [":communication_forbidden_dependencies"],
    )
