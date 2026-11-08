"""Shared production and controlled transport dependencies for the SSH broker."""

load(
    "//build_system/architecture:topology.bzl",
    "CAPNP_CODEC",
    "CAPNP_RPC",
    "CAPNP_SCHEMA",
    "TRANSPORT_RUNTIME_RPC",
    "CAPNP_VALUE",
    "YOLO_AGENT_CORE",
    "YOLO_APPLICATION_SUPPORT",
    "YOLO_APPLICATION_SUPPORT_TEST_SUPPORT",
    "YOLO_BROKER_HUB_PROTOCOL",
    "YOLO_CLI",
    "YOLO_CLIENT",
    "YOLO_CRYPTO",
    "YOLO_LEASE",
    "YOLO_LOCAL_UNIX_TEST_SUPPORT",
    "YOLO_MAINTENANCE",
    "YOLO_PATHS",
    "YOLO_PRIVATE_ACL",
    "YOLO_PROCESS",
    "YOLO_PROTOCOL_VALIDATION",
    "YOLO_REMOTE",
    "YOLO_REMOTE_PROVISIONING_PROTOCOL",
    "YOLO_RUNTIME_PROTOCOL",
    "YOLO_SESSION_TIMING",
    "YOLO_TERMINAL_MODEL",
    "YOLO_TRANSPORT_CORE",
    "YOLO_WIRE",
)


_REMOTE = YOLO_REMOTE
_REMOTE_TEST = "//core/crates/transport/remote:yolo_remote_broker_test_support"
_TUNNEL = "//core/crates/transport/broker/tunnel:yolo_broker_tunnel"
_TUNNEL_TEST = "//core/crates/transport/broker/tunnel:yolo_broker_tunnel_test_support"
_PROVISION = "//core/crates/transport/remote/provisioning:yolo_remote_provisioning"
_PROVISION_TEST = "//core/crates/transport/remote/provisioning:broker_test_support"
_APPLICATION = YOLO_APPLICATION_SUPPORT
_APPLICATION_TEST = YOLO_APPLICATION_SUPPORT_TEST_SUPPORT
_LOCAL_TEST = YOLO_LOCAL_UNIX_TEST_SUPPORT

REMOTE_BROKER_DEPS = [
    TRANSPORT_RUNTIME_RPC,
    CAPNP_RPC,
    CAPNP_CODEC,
    "//core/crates/transport/capnp/sensitive",
    CAPNP_SCHEMA,
    _PROVISION,
    YOLO_REMOTE_PROVISIONING_PROTOCOL,
    YOLO_BROKER_HUB_PROTOCOL,
    _TUNNEL,
    _APPLICATION,
    YOLO_AGENT_CORE,
    YOLO_LEASE,
    YOLO_MAINTENANCE,
    YOLO_PATHS,
    YOLO_PROCESS,
    YOLO_RUNTIME_PROTOCOL,
    YOLO_TERMINAL_MODEL,
    YOLO_PROTOCOL_VALIDATION,
    YOLO_WIRE,
    YOLO_CLI,
    YOLO_CRYPTO,
    YOLO_PRIVATE_ACL,
    YOLO_CLIENT,
    YOLO_TRANSPORT_CORE,
    _REMOTE,
    YOLO_SESSION_TIMING,
    "@crates//:anyhow",
    "@crates//:futures",
    "@crates//:libc",
    "@crates//:russh",
    "@crates//:serde",
    "@crates//:serde_bytes",
    "@crates//:serde_json",
    "@crates//:thiserror",
    "@crates//:tokio",
    "@crates//:tracing",
    "@crates//:uuid",
    "@crates//:zeroize",
]
REMOTE_BROKER_TEST_ALIASES = {
    _PROVISION_TEST: "yolo_remote_provisioning",
    _TUNNEL_TEST: "yolo_broker_tunnel",
    _APPLICATION_TEST: "yolo_application_support",
    _LOCAL_TEST: "yolo_local_unix",
    _REMOTE_TEST: "yolo_remote",
}
REMOTE_BROKER_TEST_DEPS = [dependency for dependency in REMOTE_BROKER_DEPS if dependency not in [_REMOTE, _TUNNEL, _PROVISION, _APPLICATION]] + [
    "//core:yolo_runtime",
    "//core/crates/transport/remote/provisioning/protocol:test_support",
    _PROVISION_TEST,
    _TUNNEL_TEST,
    "//core/crates/transport/broker/hub/protocol:test_support",
    _APPLICATION_TEST,
    _LOCAL_TEST,
    _REMOTE_TEST,
    "@crates//:base64",
    "@crates//:capnp",
    "@crates//:capnp-rpc",
    "@crates//:hmac",
    CAPNP_VALUE,
    "@crates//:russh-sftp",
    "@crates//:sha1",
    "@crates//:tempfile",
]
