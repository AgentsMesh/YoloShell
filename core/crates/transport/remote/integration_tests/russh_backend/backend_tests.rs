#[path = "backend_tests/agent.rs"]
mod agent;
#[path = "backend_tests/approved_channel_tests.rs"]
mod approved_channel_tests;
#[path = "backend_tests/auth_limit_tests.rs"]
mod auth_limit_tests;
#[path = "backend_tests/bundle_executable_tests.rs"]
mod bundle_executable_tests;
#[path = "../../test_support/russh_backend/bundle_fixture.rs"]
mod bundle_fixture;
#[path = "backend_tests/bundle_retention_capacity_tests.rs"]
mod bundle_retention_capacity_tests;
#[path = "backend_tests/bundle_retention_tests.rs"]
mod bundle_retention_tests;
#[path = "backend_tests/bundle_selection_tests.rs"]
mod bundle_selection_tests;
#[path = "backend_tests/bundle_stage_tests.rs"]
mod bundle_stage_tests;
#[path = "backend_tests/certificate_auth_tests.rs"]
mod certificate_auth_tests;
#[path = "backend_tests/channel_open_custody_tests.rs"]
mod channel_open_custody_tests;
#[path = "backend_tests/channel_open_registry_retention_tests.rs"]
mod channel_open_registry_retention_tests;
#[path = "backend_tests/command_cancellation_tests.rs"]
mod command_cancellation_tests;
#[path = "backend_tests/command_terminal_tests.rs"]
mod command_terminal_tests;
#[path = "backend_tests/connection_cleanup_tests.rs"]
mod connection_cleanup_tests;
#[path = "../../test_support/russh_backend/constants.rs"]
mod constants;
#[path = "backend_tests/deployment_adversarial_tests.rs"]
mod deployment_adversarial_tests;
#[path = "backend_tests/deployment_branch_tests.rs"]
mod deployment_branch_tests;
#[path = "backend_tests/deployment_cancellation_tests.rs"]
mod deployment_cancellation_tests;
#[path = "backend_tests/deployment_custody_error_tests.rs"]
mod deployment_custody_error_tests;
#[path = "backend_tests/deployment_durability_tests.rs"]
mod deployment_durability_tests;
#[path = "backend_tests/deployment_error_tests.rs"]
mod deployment_error_tests;
#[path = "backend_tests/deployment_reconnect_tests.rs"]
mod deployment_reconnect_tests;
#[path = "backend_tests/deployment_security_tests.rs"]
mod deployment_security_tests;
#[path = "backend_tests/deployment_support.rs"]
mod deployment_support;
#[path = "backend_tests/deployment_tests.rs"]
mod deployment_tests;
#[path = "backend_tests/failure_tests.rs"]
mod failure_tests;
#[path = "../../test_support/russh_backend/fixture.rs"]
mod fixture;
#[path = "backend_tests/hub_peer_cancellation_tests.rs"]
mod hub_peer_cancellation_tests;
#[path = "../../test_support/russh_backend/hub_peer_fixture.rs"]
mod hub_peer_fixture;
#[path = "backend_tests/hub_peer_tests.rs"]
mod hub_peer_tests;
#[path = "backend_tests/keepalive_tests.rs"]
mod keepalive_tests;
#[path = "../../test_support/russh_backend/memory_fs.rs"]
mod memory_fs;
#[path = "backend_tests/operation_timeout_tests.rs"]
mod operation_timeout_tests;
#[path = "backend_tests/posix_failure_tests.rs"]
mod posix_failure_tests;
#[path = "../../test_support/russh_backend/server.rs"]
mod server;
#[path = "../../test_support/russh_backend/server_channel_open.rs"]
mod server_channel_open;
#[path = "../../test_support/russh_backend/server_command.rs"]
mod server_command;
#[path = "../../test_support/russh_backend/server_command_cleanup.rs"]
mod server_command_cleanup;
#[path = "../../test_support/russh_backend/server_hub_peer.rs"]
mod server_hub_peer;
#[path = "../../test_support/russh_backend/server_identity.rs"]
mod server_identity;
#[path = "../../test_support/russh_backend/server_protocol.rs"]
mod server_protocol;
#[path = "backend_tests/sftp_attribute_tests.rs"]
mod sftp_attribute_tests;
#[path = "../../test_support/russh_backend/sftp_extensions.rs"]
mod sftp_extensions;
#[path = "backend_tests/sftp_handle_model_tests.rs"]
mod sftp_handle_model_tests;
#[path = "backend_tests/sftp_initialization_custody_tests.rs"]
mod sftp_initialization_custody_tests;
#[path = "backend_tests/sftp_lifecycle_custody_tests.rs"]
mod sftp_lifecycle_custody_tests;
#[path = "backend_tests/sftp_read_race_tests.rs"]
mod sftp_read_race_tests;
#[path = "backend_tests/sftp_read_support.rs"]
mod sftp_read_support;
#[path = "../../test_support/russh_backend/sftp_response.rs"]
mod sftp_response;
#[path = "../../test_support/russh_backend/sftp_server.rs"]
mod sftp_server;
#[path = "backend_tests/sftp_transfer_custody_adversarial_tests.rs"]
mod sftp_transfer_custody_adversarial_tests;
#[path = "../../test_support/russh_backend/sftp_wire.rs"]
mod sftp_wire;
#[path = "backend_tests/tcp_connect_tests.rs"]
mod tcp_connect_tests;

include!("backend_tests/scenarios.rs");
