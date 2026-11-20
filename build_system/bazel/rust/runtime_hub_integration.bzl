"""Public Hub clients and federation dependencies for live runtime acceptance."""

load(
    "//build_system/architecture:topology.bzl",
    "CAPNP_ARGUMENT",
    "CAPNP_CODEC",
    "CAPNP_RPC",
    "TRANSPORT_RUNTIME_RPC",
    "TRANSPORT_RUNTIME_TESTING",
    "YOLO_AGENT_CONTROL_CONTRACT",
    "YOLO_BROKER_HUB_PROTOCOL",
    "YOLO_CRYPTO",
    "YOLO_HUB_CLIENT",
    "YOLO_HUB_CONNECTOR_CLIENT",
    "YOLO_HUB_IDENTITY",
    "YOLO_HUB_JSONRPC_PROTOCOL",
    "YOLO_HUB_JSONRPC_TRANSPORT",
    "YOLO_HUB_LOCAL_BACKEND",
    "YOLO_HUB_RPC_PROTOCOL",
    "YOLO_PATHS",
    "YOLO_PRESENTATION_LAYOUT",
    "YOLO_PRESENTATION_PROTOCOL",
    "YOLO_REMOTE_PROVISIONING_PROTOCOL",
    "YOLO_TUI_TAB",
)


HUB_INTEGRATION_DEPS = [
    TRANSPORT_RUNTIME_RPC,
    TRANSPORT_RUNTIME_TESTING,
    CAPNP_RPC,
    CAPNP_CODEC,
    "//core/crates/hub/relay/control/protocol:yolo_hub_relay_control_protocol",
    CAPNP_ARGUMENT,
    "@crates//:ring",
    "//core/crates/hub/client/access/protocol:yolo_hub_client_access_protocol",
    "//core/crates/hub/client/access/registry:yolo_hub_client_access_registry",
    "//core/crates/hub/client/transport:yolo_hub_client_transport",
    "//core/crates/hub/ssh/entry:yolo_hub_ssh_entry",
    "//core/crates/hub/public/client/server:yolo_hub_public_client_server",
    "//core/crates/application/local/profile:yolo_local_profile",
    "//clients/tui/local/startup:yolo_tui_local_startup",
    "@crates//:tracing-subscriber",
    "//core/crates/hub/observation/store:yolo_hub_observation_store",
    "//core/crates/hub/peer/dialer:yolo_hub_peer_dialer",
    "//core/crates/hub/peer/protocol:yolo_hub_peer_protocol",
    "//core/crates/hub/remote/setup/protocol:yolo_hub_remote_setup_protocol",
    YOLO_REMOTE_PROVISIONING_PROTOCOL,
    YOLO_BROKER_HUB_PROTOCOL,
    "//core/crates/hub/ssh/profile/protocol:yolo_hub_ssh_profile_protocol",
    YOLO_CRYPTO,
    "//core/crates/distribution/bundle:yolo_distribution_bundle",
    "//core/crates/distribution/publication/protocol:yolo_distribution_publication_protocol",
    "//core/crates/hub/build/protocol:yolo_hub_build_protocol",
    "//integration_tests/runtime/terminal_query_fixture:library",
    YOLO_AGENT_CONTROL_CONTRACT,
    YOLO_HUB_CONNECTOR_CLIENT,
    "//core/crates/hub/connector/protocol:yolo_hub_connector_protocol",
    "//core/crates/hub/connector/service:yolo_hub_connector_service",
    "//core/crates/hub/connector/store:yolo_hub_connector_store",
    YOLO_HUB_CLIENT,
    "//core/crates/hub/catalog/client:yolo_hub_catalog_client",
    "//core/crates/hub/file/client:yolo_hub_file_client",
    "//core/crates/hub/file/protocol:yolo_hub_file_protocol",
    "//core/crates/hub/file/executor:test_support",
    "//core/crates/hub/delegation:yolo_hub_delegation",
    "//core/crates/hub/federation:yolo_hub_federation",
    "//core/crates/hub/dispatch:yolo_hub_dispatch",
    "//core/crates/hub/federation/settings:yolo_hub_federation_settings",
    "//core/crates/hub/agent/protocol:yolo_hub_agent_protocol",
    "//core/crates/hub/routing:yolo_hub_routing",
    YOLO_HUB_IDENTITY,
    YOLO_HUB_LOCAL_BACKEND,
    "//core/crates/hub/local/startup:yolo_hub_local_startup",
    "//core/crates/hub/management/protocol:yolo_hub_management_protocol",
    "//core/crates/hub/management/runtime:yolo_hub_management_runtime",
    YOLO_HUB_RPC_PROTOCOL,
    "//core/crates/hub/server:yolo_hub_server",
    "//core/crates/hub/terminal/protocol:yolo_hub_terminal_protocol",
    "//core/crates/hub/state:yolo_hub_state",
    "//core/crates/runtime/engine/startup/guard:yolo_startup_guard",
    "//core/crates/hub/terminal/client:yolo_hub_terminal_client",
    YOLO_HUB_JSONRPC_PROTOCOL,
    YOLO_HUB_JSONRPC_TRANSPORT,
    YOLO_TUI_TAB,
    "//clients/tui/creation/journal:yolo_tui_creation_journal",
    YOLO_PRESENTATION_LAYOUT,
    YOLO_PRESENTATION_PROTOCOL,
    YOLO_PATHS,
]

_TUI_BOOTSTRAP_FIXTURE = "//clients/tui/local/startup:bootstrap_fixture"
_STARTUP_PARENT = "//core/crates/runtime/engine/startup/guard:startup_parent"
_FILE_HELPER = "//core/crates/application/yolo:yolo"
_FILE_FAULT_HELPER = "//core/crates/hub/file/executor:fault_helper"
_BUNDLE_HOST = "//core/crates/application/yolo:yolo"
HUB_BUNDLE_TEST_OBJECTS = [_BUNDLE_HOST]
HUB_FILE_TEST_OBJECTS = [_FILE_HELPER, _FILE_FAULT_HELPER]
_CONNECTOR_FIXTURE = "//integration_tests/runtime/connector_fixture:yolo-test-connector"
_QUERY_FIXTURE = "//integration_tests/runtime/terminal_query_fixture:yolo-test-terminal-query"
HUB_QUERY_TEST_OBJECTS = [_QUERY_FIXTURE]
HUB_CONNECTOR_TEST_OBJECTS = [_CONNECTOR_FIXTURE]
CONNECTOR_FIXTURE_ARCHITECTURE_DATA = ["//integration_tests/runtime/connector_fixture:architecture_check_data"]
_NATIVE_HUB_PROBE_DATA = select({
    "@platforms//os:macos": [_TUI_BOOTSTRAP_FIXTURE],
    "//conditions:default": [],
})

def native_hub_probe_data():
    name = "native_hub_probe_data"
    native.filegroup(
        name = name,
        testonly = True,
        srcs = _NATIVE_HUB_PROBE_DATA + depset([_STARTUP_PARENT] + HUB_FILE_TEST_OBJECTS +
               HUB_CONNECTOR_TEST_OBJECTS + HUB_QUERY_TEST_OBJECTS + HUB_BUNDLE_TEST_OBJECTS).to_list(),
    )
    return ":" + name

def native_hub_environment(base):
    parent = Label(_STARTUP_PARENT)
    bootstrap_fixture = Label(_TUI_BOOTSTRAP_FIXTURE)
    file_helper = Label(_FILE_HELPER)
    file_fault = Label(_FILE_FAULT_HELPER)
    connector = Label(_CONNECTOR_FIXTURE)
    query = Label(_QUERY_FIXTURE)
    bundle_host = Label(_BUNDLE_HOST)
    return dict(
        base,
        YOLO_TEST_STARTUP_PARENT_BINARY = "{}/{}".format(parent.package, parent.name),
        YOLO_TEST_TUI_BOOTSTRAP_FIXTURE_BINARY = "{}/{}".format(bootstrap_fixture.package, bootstrap_fixture.name),
        YOLO_TEST_FILE_HELPER_BINARY = "{}/{}".format(file_helper.package, file_helper.name),
        YOLO_TEST_FILE_FAULT_BINARY = "{}/{}".format(file_fault.package, file_fault.name),
        YOLO_TEST_CONNECTOR_BINARY = "{}/{}".format(connector.package, connector.name),
        YOLO_TEST_TERMINAL_QUERY_BINARY = "{}/{}".format(query.package, query.name),
        YOLO_TEST_BUNDLE_HOST_BINARY = "{}/{}".format(bundle_host.package, bundle_host.name),
    )
