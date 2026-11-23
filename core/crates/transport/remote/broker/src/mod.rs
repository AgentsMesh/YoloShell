#[path = "broker_paths/broker_paths.rs"]
mod broker_paths;
#[path = "client/client.rs"]
mod client;
#[path = "client/client_io.rs"]
mod client_io;
#[path = "config/config.rs"]
mod config;
#[path = "config/config_artifact.rs"]
mod config_artifact;
#[path = "config/config_generation_pending.rs"]
mod config_generation_pending;
#[path = "config/config_resolve.rs"]
mod config_resolve;
#[path = "config/config_snapshot.rs"]
mod config_snapshot;
#[path = "config/config_snapshot_passphrases.rs"]
mod config_snapshot_passphrases;
#[path = "config/config_validation.rs"]
mod config_validation;
#[path = "connection/connection.rs"]
mod connection;
#[path = "connection/connection_bootstrap.rs"]
mod connection_bootstrap;
#[path = "connection/connection_factory.rs"]
mod connection_factory;
#[path = "connection/connection_hub_peer.rs"]
mod connection_hub_peer;
mod provisioning_rpc;
#[path = "connection/connection_runtime.rs"]
mod connection_runtime;
#[path = "constants.rs"]
mod constants;
#[cfg(test)]
#[path = "coordination_hook.rs"]
mod coordination_hook;
#[path = "identity/credential_binding.rs"]
mod credential_binding;
#[path = "deadline_fence.rs"]
mod deadline_fence;
#[path = "server/factory.rs"]
mod factory;
#[path = "handlers/handlers.rs"]
mod handlers;
#[path = "handoff/handoff.rs"]
mod handoff;
#[path = "handoff/handoff_endpoint.rs"]
mod handoff_endpoint;
#[path = "hub_peer/hub_peer_constants.rs"]
mod hub_peer_constants;
#[path = "hub_peer/hub_peer_proxy.rs"]
mod hub_peer_proxy;
#[path = "hub_peer/hub_peer_response.rs"]
mod hub_peer_response;
#[path = "identity/identity_passphrase_binding.rs"]
mod identity_passphrase_binding;
#[path = "identity/identity_passphrase/constants.rs"]
mod identity_passphrase_constants;
#[path = "identity/identity_passphrases.rs"]
mod identity_passphrases;
#[path = "instance_lock.rs"]
mod instance_lock;
#[path = "launch/launch_authority.rs"]
mod launch_authority;
#[path = "launch/launch_deadline.rs"]
mod launch_deadline;
#[path = "runtime/maintenance_runtime.rs"]
mod maintenance_runtime;
#[path = "namespace.rs"]
mod namespace;
#[path = "operations.rs"]
mod operations;
#[path = "protocol/protocol.rs"]
mod protocol;
#[path = "publication_gate.rs"]
mod publication_gate;
#[path = "retirement.rs"]
mod retirement;
#[path = "revision.rs"]
mod revision;
#[path = "runtime/runtime_forward.rs"]
mod runtime_forward;
#[path = "runtime/runtime_proxy.rs"]
mod runtime_proxy;
#[path = "runtime/runtime_rpc.rs"]
mod runtime_rpc;
#[path = "runtime/runtime_admission.rs"]
mod runtime_admission;
#[path = "security/security.rs"]
mod security;
#[path = "security/security_file.rs"]
mod security_file;
#[path = "server/server.rs"]
mod server;
#[path = "server/server_admission.rs"]
mod server_admission;
#[path = "socket/socket_bind_at.rs"]
mod socket_bind_at;
#[path = "socket/socket_binding.rs"]
mod socket_binding;
#[path = "socket/socket_directory.rs"]
mod socket_directory;
#[path = "socket/socket_directory_io.rs"]
mod socket_directory_io;
#[path = "socket/socket_identity.rs"]
mod socket_identity;
#[path = "socket/socket_probe.rs"]
mod socket_probe;
#[path = "socket/sockets.rs"]
mod sockets;
#[path = "supervision/supervision_constants.rs"]
mod supervision_constants;
#[path = "supervision/supervision_containment.rs"]
mod supervision_containment;
#[path = "supervision/supervision_store.rs"]
mod supervision_store;
#[path = "timing.rs"]
mod timing;

pub use broker_paths::{BrokerPaths, BrokerStateAnchor};
pub use client::{BrokerClient, BrokerClientError, BrokerTransition};
pub use config::BrokerConfig;
pub use yolo_broker_hub_protocol::{
    BrokerHubConnectionBinding, BrokerHubError, BrokerHubOpenRequest, BrokerHubOpened,
    BrokerHubProfile, HUB_TRANSPORT_CAPABILITY,
};
pub use yolo_broker_tunnel::{BrokerHubTunnel, BrokerTunnelClose};
pub fn validate_broker_host_id(value: &str) -> anyhow::Result<()> {
    config_validation::validate_host_id(value)
}
pub fn validate_broker_config_revision(value: &str) -> anyhow::Result<()> {
    config_validation::validate_config_revision(value)
}
pub use config_snapshot::{
    BrokerConfigGeneration, stage_broker_config, stage_broker_config_guarded,
};
pub use config_snapshot_passphrases::stage_broker_config_guarded_with_passphrases;
pub use connection::BrokerConnectionPool;
pub use connection_factory::{BrokerConnectFuture, BrokerConnectionFactory};
pub use constants::{
    BROKER_PROTOCOL_VERSION, BROKER_READY_POLL_INTERVAL, BROKER_SPAWN_RETRY_INTERVAL,
    CONNECTION_BOOTSTRAP_CAPABILITY, DEFAULT_COMMAND_OUTPUT_BYTES, DEFAULT_COMMAND_TIMEOUT_MILLIS,
    DEFAULT_FILE_READ_BYTES, WORKING_DIRECTORY_CAPABILITY,
};
pub use handoff::{BrokerReconcileGuard, BrokerRetirement, broker_instance_available};
pub use identity_passphrases::{IDENTITY_PASSPHRASE_WIRE_VERSION, IdentityPassphrases};
pub use launch_authority::{
    AuthorizedBrokerStartup, BrokerLaunchAuthority, authorize_broker_launch,
};
pub use launch_deadline::BrokerLaunchDeadline;
pub use protocol::{
    BrokerCommandRequest, BrokerFileRequest, BrokerRequest, BrokerRequestEnvelope, BrokerResponse,
    BrokerResult, BrokerStatus,
};
pub use retirement::retire_broker;
pub use server::{BrokerShutdownErrors, RemoteBroker};
pub use server_admission::BrokerAdoptionAcknowledgement;
pub use sockets::{validate_broker_socket, validate_broker_socket_path};
pub use supervision_constants::BROKER_SUPERVISION_RECORD_VERSION;
pub use supervision_store::BrokerSupervisionStore;
pub use timing::{
    BROKER_CLIENT_OPERATION_TIMEOUT, BROKER_READY_TIMEOUT, EXISTING_SOCKET_CLIENT_OPERATION_TIMEOUT,
};

pub fn existing_runtime_client(socket: impl Into<std::path::PathBuf>) -> crate::Client {
    crate::Client::new(socket).with_operation_timeout(EXISTING_SOCKET_CLIENT_OPERATION_TIMEOUT)
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../test_support/integration_tests.rs"]
#[cfg(test)]
mod integration_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/mod.rs"]
mod tests;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/client_aba_tests.rs"]
mod client_aba_tests;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/deadline_fence_lock_tests.rs"]
mod deadline_fence_lock_tests;
